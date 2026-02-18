//! STARK circuit (AIR) for vouch verification
//!
//! This module defines the Algebraic Intermediate Representation (AIR) for
//! proving vouch verification calculations using STARKs.
//!
//! The circuit proves:
//! - Effective_Vouches = |Vouchers| - |Voucher_Flaggers|
//! - Regular_Flags = |Flaggers| - |Voucher_Flaggers|
//! - Standing = Effective_Vouches - Regular_Flags

use winter_math::ToElements;
use winterfell::{
    math::{fields::f128::BaseElement, FieldElement},
    Air, AirContext, Assertion, EvaluationFrame, ProofOptions, TraceInfo,
    TransitionConstraintDegree,
};

pub const TRACE_WIDTH: usize = 8;

/// The AIR for vouch verification
///
/// Execution trace layout (8 columns):
/// - Column 0: voucher_count (cumulative count of vouchers)
/// - Column 1: flagger_count (cumulative count of flaggers)
/// - Column 2: intersection_count (cumulative count of voucher-flaggers)
/// - Column 3: effective_vouches (voucher_count - intersection_count)
/// - Column 4: regular_flags (flagger_count - intersection_count)
/// - Column 5: standing (effective_vouches - regular_flags)
/// - Column 6-7: auxiliary columns for constraints
pub struct VouchAir {
    context: AirContext<BaseElement>,
    pub_inputs: VouchPublicInputs,
}

impl Air for VouchAir {
    type BaseField = BaseElement;
    type PublicInputs = VouchPublicInputs;

    fn new(trace_info: TraceInfo, pub_inputs: Self::PublicInputs, options: ProofOptions) -> Self {
        // Define constraint degrees
        let degrees = vec![
            TransitionConstraintDegree::new(1), // voucher_count increment
            TransitionConstraintDegree::new(1), // flagger_count increment
            TransitionConstraintDegree::new(1), // intersection_count increment
            TransitionConstraintDegree::new(1), // effective_vouches calculation
            TransitionConstraintDegree::new(1), // regular_flags calculation
            TransitionConstraintDegree::new(1), // standing calculation
        ];

        let context = AirContext::new(trace_info, degrees, 6, options);
        Self {
            context,
            pub_inputs,
        }
    }

    fn context(&self) -> &AirContext<Self::BaseField> {
        &self.context
    }

    fn evaluate_transition<E: FieldElement + From<Self::BaseField>>(
        &self,
        frame: &EvaluationFrame<E>,
        _periodic_values: &[E],
        result: &mut [E],
    ) {
        let current = frame.current();
        let next = frame.next();

        // Constraint 0: voucher_count increments by 0 or 1
        let voucher_delta = next[0] - current[0];
        result[0] = voucher_delta * (voucher_delta - E::ONE);

        // Constraint 1: flagger_count increments by 0 or 1
        let flagger_delta = next[1] - current[1];
        result[1] = flagger_delta * (flagger_delta - E::ONE);

        // Constraint 2: intersection_count increments by 0 or 1
        let intersection_delta = next[2] - current[2];
        result[2] = intersection_delta * (intersection_delta - E::ONE);

        // Constraint 3: effective_vouches = voucher_count - intersection_count
        result[3] = next[3] - (next[0] - next[2]);

        // Constraint 4: regular_flags = flagger_count - intersection_count
        result[4] = next[4] - (next[1] - next[2]);

        // Constraint 5: standing = effective_vouches - regular_flags
        result[5] = next[5] - (next[3] - next[4]);
    }

    fn get_assertions(&self) -> Vec<Assertion<Self::BaseField>> {
        let last_step = self.trace_length() - 1;

        vec![
            // Initial values (step 0)
            Assertion::single(0, 0, BaseElement::ZERO), // voucher_count starts at 0
            Assertion::single(1, 0, BaseElement::ZERO), // flagger_count starts at 0
            Assertion::single(2, 0, BaseElement::ZERO), // intersection_count starts at 0
            Assertion::single(3, 0, BaseElement::ZERO), // effective_vouches starts at 0
            Assertion::single(4, 0, BaseElement::ZERO), // regular_flags starts at 0
            Assertion::single(5, 0, BaseElement::ZERO), // standing starts at 0
            // Final values (last step)
            Assertion::single(
                3,
                last_step,
                BaseElement::new(self.pub_inputs.effective_vouches as u128),
            ),
            Assertion::single(
                4,
                last_step,
                BaseElement::new(self.pub_inputs.regular_flags as u128),
            ),
            Assertion::single(
                5,
                last_step,
                // Encode signed standing as unsigned
                BaseElement::new((self.pub_inputs.standing as i64 + (1i64 << 31)) as u128),
            ),
        ]
    }
}

impl VouchAir {
    /// Get the trace length
    pub fn trace_length(&self) -> usize {
        self.context.trace_len()
    }
}

/// Public inputs for vouch verification proof
#[derive(Debug, Clone)]
pub struct VouchPublicInputs {
    pub effective_vouches: usize,
    pub regular_flags: usize,
    pub standing: i32,
}

impl ToElements<BaseElement> for VouchPublicInputs {
    fn to_elements(&self) -> Vec<BaseElement> {
        vec![
            BaseElement::new(self.effective_vouches as u128),
            BaseElement::new(self.regular_flags as u128),
            // Standing can be negative, so we encode it as unsigned
            BaseElement::new((self.standing as i64 + (1i64 << 31)) as u128),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vouch_air_creation() {
        let pub_inputs = VouchPublicInputs {
            effective_vouches: 2,
            regular_flags: 1,
            standing: 1,
        };

        let trace_info = TraceInfo::new(TRACE_WIDTH, 8);
        let options = ProofOptions::new(
            32, // number of queries
            8,  // blowup factor
            0,  // grinding factor
            winterfell::FieldExtension::None,
            4,   // FRI folding factor
            255, // FRI max remainder degree (must be 2^n - 1)
            winterfell::BatchingMethod::Linear,
            winterfell::BatchingMethod::Linear,
        );

        let air = VouchAir::new(trace_info, pub_inputs, options);
        assert_eq!(air.trace_length(), 8);
    }

    #[test]
    fn test_public_inputs_encoding() {
        let inputs = VouchPublicInputs {
            effective_vouches: 3,
            regular_flags: 2,
            standing: -1,
        };

        let elements = inputs.to_elements();
        assert_eq!(elements.len(), 3);
        assert_eq!(elements[0], BaseElement::new(3));
        assert_eq!(elements[1], BaseElement::new(2));
        // Negative standing encoded as unsigned
        assert_eq!(
            elements[2],
            BaseElement::new((-1i64 + (1i64 << 31)) as u128)
        );
    }
}
