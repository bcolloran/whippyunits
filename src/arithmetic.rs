#[macro_export]
#[doc(hidden)]
macro_rules! scalar_quantity_mul_div_interface {
    (
        ($($single_dimension_single_scale_params:tt)*),
        ($($inversion_params:tt)*),
        ($($inversion_where_clauses:tt)*),
        $T:ty
    ) => {
        impl<
            $($single_dimension_single_scale_params)*
        >
            core::ops::Mul<$crate::quantity_type!($T)> for $T
        {
            type Output = $crate::quantity_type!($T);

            fn mul(self: $T, other: Self::Output) -> Self::Output {
                let result_value = self * other.unsafe_value;
                Self::Output::new(result_value)
            }
        }

        impl<
            $($single_dimension_single_scale_params)*
            $($inversion_params)*
        >
            core::ops::Div<$crate::quantity_type!($T)> for $T
        where
            $($inversion_where_clauses)*
        {
            type Output = $crate::inverse_quantity_type!($T);

            fn div(self: $T, other: $crate::quantity_type!($T)) -> Self::Output {
                let result_value = self / other.unsafe_value;
                Self::Output::new(result_value)
            }
        }
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! quantity_scalar_mul_div_interface {
    (($($single_dimension_single_scale_params:tt)*), $op:tt, $fn:ident, $trait:ident, $T:ty) => {
        impl<
            $($single_dimension_single_scale_params)*
        >
            core::ops::$trait<$T> for $crate::quantity_type!($T)
        {
            type Output = Self;

            fn $fn(self, other: $T) -> Self::Output {
                Self::new(self.unsafe_value $op other)
            }
        }
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! quantity_scalar_mul_div_assign_interface {
    (($($single_dimension_single_scale_params:tt)*), $op:tt, $fn:ident, $trait:ident, $T:ty) => {
        impl<
            $($single_dimension_single_scale_params)*
        >
            core::ops::$trait<$T> for $crate::quantity_type!($T)
        {
            fn $fn(&mut self, other: $T) {
                self.unsafe_value $op other;
            }
        }
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! quantity_quantity_add_sub_interface {
    // Scale-strict interface (measurement scales must match)
    (
        ($($single_dimension_single_scale_params:tt)*),
        $op:tt, $fn:ident, $trait:ident, $T:ty, $rescale_fn:ident
    ) => {
        impl<
            $($single_dimension_single_scale_params)*
        >
            core::ops::$trait<$crate::quantity_type!($T)>
            for $crate::quantity_type!($T)
        {
            type Output = Self;

            fn $fn(
                self,
                other: Self,
            ) -> Self::Output {
                Self::new(self.unsafe_value $op other.unsafe_value)
            }
        }
    };
}

// AddAssign/SubAssign are scale-strict
#[macro_export]
#[doc(hidden)]
macro_rules! quantity_quantity_add_sub_assign_interface {
    // Scale-strict interface (measurement scales must match)
    (
        ($($single_dimension_single_scale_params:tt)*),
        $op:tt, $fn:ident, $trait:ident, $T:ty, $rescale_fn:ident
    ) => {
        impl<
            $($single_dimension_single_scale_params)*
        >
            core::ops::$trait<
                $crate::quantity_type!($T),
            > for $crate::quantity_type!($T)
        {
            fn $fn(&mut self, other: $crate::quantity_type!($T)) {
                self.unsafe_value $op other.unsafe_value;
            }
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! quantity_quantity_mul_div_interface {
    // Strict interface (measurement scales must match) (only one set of scale parameters)
    (
        ($($multiple_dimension_multiple_scale_params:tt)*),
        ($($output_dimension_where_clauses:tt)*),
        $op:tt, $log_op:tt, $fn:ident, $trait:ident, $T:ty, $rescale_fn:ident
    ) => {
        impl<
            $($multiple_dimension_multiple_scale_params)*
        >
            core::ops::$trait<
                $crate::multiplication_input!(RightHand, $T),
            >
            for $crate::multiplication_input!(LeftHand, $T)
        where
            $($output_dimension_where_clauses)*
        {
            type Output = $crate::multiplication_output!($T, $log_op);

            fn $fn(
                self,
                other: $crate::multiplication_input!(RightHand, $T),
            ) -> Self::Output {
                Self::Output::new(self.unsafe_value $op other.unsafe_value)
            }
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! quantity_neg_interface {
    (($($single_dimension_single_scale_params:tt)*), $T:ty) => {
        impl<
            $($single_dimension_single_scale_params)*
        >
            core::ops::Neg for $crate::quantity_type!($T)
        where
            $T: core::ops::Neg<Output = $T>
        {
            type Output = Self;

            fn neg(self) -> Self::Output {
                Self::new(-self.unsafe_value)
            }
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! quantity_quantity_partial_ord_interface {
    // Scale-strict comparison interface (measurement scales must match)
    (
        ($($single_dimension_single_scale_params:tt)*),
        $T:ty, $rescale_fn:ident
    ) => {
        impl<
            $($single_dimension_single_scale_params)*
        >
            core::cmp::PartialOrd<$crate::quantity_type!($T)>
            for $crate::quantity_type!($T)
        where
            $T: PartialOrd,
            Brand: PartialEq,
        {
            fn partial_cmp(&self, other: &$crate::quantity_type!($T)) -> Option<::core::cmp::Ordering> {
                self.unsafe_value.partial_cmp(&other.unsafe_value)
            }
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! _define_arithmetic_signed {
    (($($single_dimension_single_scale_params:tt)*),
     ($($multiple_dimension_multiple_scale_params:tt)*),
     ($($inversion_params:tt)*),
     ($($inversion_where_clauses:tt)*),
     ($($mul_output_dimension_where_clauses:tt)*),
     ($($div_output_dimension_where_clauses:tt)*),
     $T:ty, $rescale_fn:ident) => {
        // scalar-quantity arithmetic operations
        $crate::scalar_quantity_mul_div_interface!(
            ($($single_dimension_single_scale_params)*),
            ($($inversion_params)*),
            ($($inversion_where_clauses)*),
            $T
        );

        // unary operations (only for signed types)

        // quantity-quantity arithmetic operations

        // quantity-quantity remainder (scale-strict, same as add/sub)

        // quantity-scalar remainder

        // quantity-quantity comparison operations (scale-strict)
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! _define_arithmetic {
    (($($single_dimension_single_scale_params:tt)*),
     ($($multiple_dimension_multiple_scale_params:tt)*),
     ($($inversion_params:tt)*),
     ($($inversion_where_clauses:tt)*),
     ($($mul_output_dimension_where_clauses:tt)*),
     ($($div_output_dimension_where_clauses:tt)*),
     $T:ty, $rescale_fn:ident) => {
        // scalar-quantity arithmetic operations
        $crate::scalar_quantity_mul_div_interface!(
            ($($single_dimension_single_scale_params)*),
            ($($inversion_params)*),
            ($($inversion_where_clauses)*),
            $T
        );

        // quantity-quantity arithmetic operations

        // quantity-quantity remainder (scale-strict, same as add/sub)

        // quantity-scalar remainder

        // quantity-quantity comparison operations (scale-strict)
    };
}
