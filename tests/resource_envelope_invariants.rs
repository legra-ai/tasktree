//! Reservations and actuals compose per resource: wall-clock span follows
//! the execution structure, tokens and wall work always sum, overflow is
//! a refusal, and nothing composed may exceed the budget.

use tasktree::{
    BudgetRefusal,
    BudgetResource,
    ResourceActual,
    ResourceBudget,
    ResourceComposition,
    ResourceReservation,
    TokenBudget,
    Tokens,
};

#[test]
fn sequential_reservations_sum_wall_time_and_tokens() {
    let local = ResourceReservation::new(5, Tokens::new(1));
    let children = [
        ResourceReservation::new(30, Tokens::new(3)),
        ResourceReservation::new(20, Tokens::new(4)),
    ];

    let subtree = local
        .compose_children(ResourceComposition::Sequential, &children)
        .expect("bounded sequential composition");

    assert_eq!(subtree.wall_time_ms(), 55);
    assert_eq!(subtree.tokens(), Tokens::new(8));
}

#[test]
fn parallel_reservations_take_wall_max_but_sum_tokens() {
    let local = ResourceReservation::new(5, Tokens::new(1));
    let children = [
        ResourceReservation::new(30, Tokens::new(3)),
        ResourceReservation::new(20, Tokens::new(4)),
    ];

    let subtree = local
        .compose_children(ResourceComposition::Parallel, &children)
        .expect("bounded parallel composition");

    assert_eq!(subtree.wall_time_ms(), 35);
    assert_eq!(subtree.tokens(), Tokens::new(8));
}

#[test]
fn actuals_retain_elapsed_span_and_total_work() {
    let local = ResourceActual::new(5, Tokens::new(1));
    let children = [
        ResourceActual::new(30, Tokens::new(3)),
        ResourceActual::new(20, Tokens::new(4)),
    ];

    let subtree = local
        .compose_children(ResourceComposition::Parallel, &children)
        .expect("bounded actual composition");

    assert_eq!(subtree.wall_span_ms(), 35);
    assert_eq!(subtree.wall_work_ms(), 55);
    assert_eq!(subtree.tokens(), Tokens::new(8));
}

#[test]
fn composition_refuses_overflow_per_resource() {
    let local = ResourceReservation::new(u64::MAX, Tokens::ZERO);
    assert_eq!(
        local
            .compose_children(
                ResourceComposition::Sequential,
                &[ResourceReservation::new(1, Tokens::ZERO)],
            )
            .expect_err("wall-time overflow must fail"),
        BudgetRefusal::Overflow {
            resource: BudgetResource::WallTime,
        }
    );

    let local = ResourceReservation::new(0, Tokens::new(u64::MAX));
    assert_eq!(
        local
            .compose_children(
                ResourceComposition::Parallel,
                &[ResourceReservation::new(0, Tokens::new(1))],
            )
            .expect_err("token overflow must fail"),
        BudgetRefusal::Overflow {
            resource: BudgetResource::Tokens,
        }
    );
}

#[test]
fn composed_reservation_and_actual_must_fit_the_budget() {
    let budget = ResourceBudget::new(40, Tokens::new(10));
    assert_eq!(
        budget
            .authorize_reservation(ResourceReservation::new(41, Tokens::new(8)))
            .expect_err("reservation exceeds wall budget"),
        BudgetRefusal::Exceeded {
            resource: BudgetResource::WallTime,
            required: 41,
            available: 40,
        }
    );
    assert_eq!(
        budget
            .authorize_actual(ResourceActual::new(39, Tokens::new(11)))
            .expect_err("actual exceeds token budget"),
        BudgetRefusal::Exceeded {
            resource: BudgetResource::Tokens,
            required: 11,
            available: 10,
        }
    );
    assert!(
        budget
            .authorize_reservation(ResourceReservation::new(40, Tokens::new(10)))
            .is_ok()
    );
}

#[test]
fn the_token_half_of_an_envelope_is_a_trickle_down_budget() {
    let budget = ResourceBudget::new(40, Tokens::new(10));
    assert_eq!(budget.token_budget(), TokenBudget::new(Tokens::new(10)));
}

#[test]
fn refusals_name_their_resource() {
    assert_eq!(BudgetResource::WallTime.as_str(), "wall_time");
    assert_eq!(BudgetResource::Tokens.to_string(), "tokens");
    assert_eq!(
        BudgetRefusal::Exceeded {
            resource: BudgetResource::Tokens,
            required: 3,
            available: 2,
        }
        .to_string(),
        "tokens budget exceeded: required 3, available 2"
    );
}
