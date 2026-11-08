//! Pure per-task decisions for coalescing progress observations.

/// Minimum gap between ordinary observations of the same progress run.
const MIN_PUBLISH_INTERVAL_MS: u64 = 1_000;

/// The most recently recorded publication for one task's progress.
///
/// Callers update their observed counters independently of this state. A
/// suppressed publication does not mean the observation should be discarded.
/// Deciding and recording are separate: `decide` never changes this state.
/// The state owns only the latest label and counters, not observation history.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProgressPublishState {
    // bounded: one latest label, replaced at the next recorded publication.
    label: Option<String>,
    processed: u64,
    total: Option<u64>,
    at_ms: Option<u64>,
    published_completion: bool,
}

/// Why one progress observation should be published or suppressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressPublishDecision {
    /// No publication has been recorded yet.
    First,
    /// The observation's label differs from the recorded label.
    StepChanged,
    /// The processed count decreased or the total changed.
    CounterReset,
    /// The counts match and completion has not been recorded for this run.
    Completed,
    /// The interval elapsed and the integer completion percentage changed.
    PercentChanged,
    /// The interval elapsed without a change in integer completion percentage.
    IntervalElapsed,
    /// An ordinary observation arrived before the minimum interval elapsed.
    Suppressed,
}

impl ProgressPublishDecision {
    /// Whether this observation is eligible for publication.
    #[must_use]
    pub fn publishes(self) -> bool {
        !matches!(self, Self::Suppressed)
    }

    /// Whether this is ordinary reporting rather than a progress transition.
    ///
    /// Callers can apply their separate shared publication limits to percentage
    /// and interval decisions. First observations, label changes, counter
    /// resets and completion remain distinguishable without such runtime
    /// policy here.
    #[must_use]
    pub fn is_steady_state(self) -> bool {
        matches!(self, Self::PercentChanged | Self::IntervalElapsed)
    }
}

impl ProgressPublishState {
    /// Decide publication eligibility without changing recorded state.
    ///
    /// Precedence is first observation, label change, counter decrease or total
    /// change, then first unrecorded completion. These transitions are eligible
    /// immediately. Other observations require at least 1,000 milliseconds
    /// since the recorded publication; percentage changes cannot bypass
    /// that floor. Caller-supplied time may rewind: elapsed time uses
    /// saturating subtraction.
    #[must_use]
    pub fn decide(
        &self,
        label: Option<&str>,
        processed: u64,
        total: Option<u64>,
        now_ms: u64,
    ) -> ProgressPublishDecision {
        let Some(published_at_ms) = self.at_ms else {
            return ProgressPublishDecision::First;
        };
        if self.label.as_deref() != label {
            return ProgressPublishDecision::StepChanged;
        }
        if processed < self.processed || total != self.total {
            return ProgressPublishDecision::CounterReset;
        }
        if total == Some(processed) && !self.published_completion {
            return ProgressPublishDecision::Completed;
        }
        if now_ms.saturating_sub(published_at_ms) < MIN_PUBLISH_INTERVAL_MS {
            return ProgressPublishDecision::Suppressed;
        }
        if Self::percent(processed, total) != Self::percent(self.processed, self.total) {
            return ProgressPublishDecision::PercentChanged;
        }
        ProgressPublishDecision::IntervalElapsed
    }

    /// Record the selected observation as the latest publication.
    ///
    /// This records exactly the supplied values and time, without calling
    /// `decide` or reading a clock. The label is owned; subsequent decisions
    /// can borrow a different string with the same contents. Completion
    /// remains recorded while label and total are unchanged, including when
    /// the processed count decreases. A changed label or total starts a new
    /// run for completion tracking. Repeated completion counts still become
    /// eligible for ordinary interval reporting after the minimum gap.
    pub fn record_published(
        &mut self,
        label: Option<&str>,
        processed: u64,
        total: Option<u64>,
        now_ms: u64,
    ) {
        let same_run = self.label.as_deref() == label && total == self.total;
        self.published_completion =
            (same_run && self.published_completion) || total == Some(processed);
        self.label = label.map(ToOwned::to_owned);
        self.processed = processed;
        self.total = total;
        self.at_ms = Some(now_ms);
    }

    fn percent(processed: u64, total: Option<u64>) -> Option<u128> {
        match total {
            Some(total) if total > 0 => Some(u128::from(processed) * 100 / u128::from(total)),
            _ => None,
        }
    }
}
