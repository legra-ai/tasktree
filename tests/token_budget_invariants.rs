//! A token budget attenuates to local children, reserves shares for
//! delegated ones, never hands out more than it holds, and never
//! credits back more than it reserved.

use tasktree::{
    BudgetRefusal,
    BudgetResource,
    TokenBudget,
    Tokens,
};

#[test]
fn a_zero_ceiling_is_a_budget_and_attenuates_to_zero() {
    let budget = TokenBudget::new(Tokens::ZERO);
    assert_eq!(budget.remaining(), Tokens::ZERO);
    assert_eq!(budget.attenuate(), TokenBudget::new(Tokens::ZERO));
    let (parent, child) = budget.reserve_remaining();
    assert_eq!(parent.remaining(), Tokens::ZERO);
    assert_eq!(child.ceiling(), Tokens::ZERO);
}

#[test]
fn attenuation_shows_the_remaining_pool_and_leaves_the_parent_unchanged() {
    let (parent, _) = TokenBudget::new(Tokens::new(100))
        .reserve(Tokens::new(30))
        .expect("fits");
    let child = parent.attenuate();
    assert_eq!(child.ceiling(), Tokens::new(70));
    assert_eq!(child.reserved(), Tokens::ZERO);
    assert_eq!(parent.reserved(), Tokens::new(30));
    assert_eq!(parent.remaining(), Tokens::new(70));
}

#[test]
fn a_reservation_shrinks_the_pool_by_exactly_the_share() {
    let budget = TokenBudget::new(Tokens::new(100));
    let (parent, child) = budget.reserve(Tokens::new(40)).expect("fits");
    assert_eq!(parent.ceiling(), Tokens::new(100));
    assert_eq!(parent.reserved(), Tokens::new(40));
    assert_eq!(parent.remaining(), Tokens::new(60));
    assert_eq!(child, TokenBudget::new(Tokens::new(40)));
}

#[test]
fn a_share_larger_than_the_pool_is_refused_with_exact_numbers() {
    let (parent, _) = TokenBudget::new(Tokens::new(100))
        .reserve(Tokens::new(70))
        .expect("fits");
    assert_eq!(
        parent.reserve(Tokens::new(31)).expect_err("does not fit"),
        BudgetRefusal::Exceeded {
            resource: BudgetResource::Tokens,
            required: 31,
            available: 30,
        }
    );
    assert!(parent.reserve(Tokens::new(30)).is_ok());
}

#[test]
fn reserving_the_remainder_twice_leaves_the_second_child_nothing() {
    let (parent, first) = TokenBudget::new(Tokens::new(10)).reserve_remaining();
    assert_eq!(first.ceiling(), Tokens::new(10));
    let (parent, second) = parent.reserve_remaining();
    assert_eq!(second.ceiling(), Tokens::ZERO);
    assert_eq!(parent.remaining(), Tokens::ZERO);
}

#[test]
fn releasing_a_share_restores_the_pool_and_over_release_is_refused() {
    let (parent, _) = TokenBudget::new(Tokens::new(10)).reserve_remaining();
    let restored = parent.release(Tokens::new(4)).expect("within reserved");
    assert_eq!(restored.remaining(), Tokens::new(4));
    assert_eq!(
        restored
            .release(Tokens::new(7))
            .expect_err("more than reserved"),
        BudgetRefusal::Exceeded {
            resource: BudgetResource::Tokens,
            required: 7,
            available: 6,
        }
    );
}

#[test]
fn a_budget_round_trips_and_refuses_a_reservation_above_its_ceiling() {
    let (parent, _) = TokenBudget::new(Tokens::new(5))
        .reserve(Tokens::new(2))
        .expect("fits");
    let json = serde_json::to_string(&parent).expect("serialize");
    assert_eq!(json, r#"{"ceiling":5,"reserved":2}"#);
    assert_eq!(
        serde_json::from_str::<TokenBudget>(&json).expect("deserialize"),
        parent
    );
    assert!(serde_json::from_str::<TokenBudget>(r#"{"ceiling":5,"reserved":6}"#).is_err());
}

#[test]
fn a_budget_displays_what_is_left_of_what_was_authorized() {
    let (parent, _) = TokenBudget::new(Tokens::new(9))
        .reserve(Tokens::new(4))
        .expect("fits");
    assert_eq!(parent.to_string(), "5 of 9 tokens remaining");
}
