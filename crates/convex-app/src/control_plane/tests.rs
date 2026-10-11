use convex_core::Error;

use super::finish;
use crate::{
    mock::MockUnitOfWork,
    test_support::{expect_commit, expect_rollback},
};

#[tokio::test]
async fn successful_work_commits_before_returning_its_value() {
    let mut work = MockUnitOfWork::default();
    expect_commit(&mut work);
    assert_eq!(finish(Box::new(work), Ok(42)).await.unwrap(), 42);
}

#[tokio::test]
async fn failed_work_rolls_back_and_preserves_its_error() {
    let mut work = MockUnitOfWork::default();
    expect_rollback(&mut work);
    let result = finish::<()>(Box::new(work), Err(Error::Forbidden)).await;
    assert!(matches!(result, Err(Error::Forbidden)));
}

#[tokio::test]
async fn rollback_failure_does_not_replace_the_operation_error() {
    let mut work = MockUnitOfWork::default();
    work.lifecycle
        .expect_rollback()
        .times(1)
        .returning(|| Err(anyhow::anyhow!("rollback unavailable")));
    let result = finish::<()>(Box::new(work), Err(Error::conflict("last owner"))).await;
    assert!(matches!(result, Err(Error::Conflict(message)) if message == "last owner"));
}

#[tokio::test]
async fn commit_failure_prevents_a_successful_result() {
    let mut work = MockUnitOfWork::default();
    work.lifecycle
        .expect_commit()
        .times(1)
        .returning(|| Err(anyhow::anyhow!("commit unavailable")));
    let result = finish(Box::new(work), Ok(42)).await;
    assert!(
        matches!(result, Err(Error::Infrastructure(error)) if error.to_string() == "commit unavailable")
    );
}
