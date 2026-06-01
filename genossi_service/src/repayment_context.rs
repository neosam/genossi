//! Phase 13 D-13-04 / D-13-10: Shared aggregation resolver fuer RepaymentEntry-Kontext.
//!
//! TDD RED-Phase: Tests existieren, Implementation noch nicht.

#[cfg(test)]
mod tests {
    // RED-Phase: alle Referenzen schlagen fehl, weil RepaymentContext +
    // RepaymentContextResolver + MockRepaymentContextResolver noch nicht
    // definiert sind. GREEN-Commit fuegt sie hinzu.
    use super::*;

    #[test]
    fn test_repayment_context_equality() {
        let a = RepaymentContext {
            share_count: 3,
            payout_amount: "12,00".to_string(),
            fiscal_year: 2025,
        };
        let b = RepaymentContext {
            share_count: 3,
            payout_amount: "12,00".to_string(),
            fiscal_year: 2025,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_repayment_context_clone() {
        let a = RepaymentContext {
            share_count: 5,
            payout_amount: "60,00".to_string(),
            fiscal_year: 2025,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn test_mock_repayment_context_resolver_compiles() {
        let _mock = MockRepaymentContextResolver::new();
        // Verifies automock generated MockRepaymentContextResolver successfully.
    }

    #[test]
    fn test_mock_repayment_context_resolver_has_aggregate_expect() {
        // Verifies automock generated expect_aggregate() (sync-Methode auf dem Trait).
        let mut mock = MockRepaymentContextResolver::new();
        mock.expect_aggregate().returning(|_phase, _entries, _mid| {
            Ok(RepaymentContext {
                share_count: 1,
                payout_amount: "1,00".to_string(),
                fiscal_year: 2025,
            })
        });
        // No call — only verifies that the expect_*-Setter exists at compile-time.
    }
}
