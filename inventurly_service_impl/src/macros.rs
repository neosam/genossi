#[macro_export]
macro_rules! gen_service_impl {
    // Helper to generate the trait
    (@generate_trait $dependencies:ident, $($field_name:ident : $field_type:path),*) => {
        pub trait $dependencies {
            type Context: Send + Sync + Clone + Eq + std::fmt::Debug + 'static;
            type Transaction: inventurly_dao::Transaction + Send + Sync + Clone + std::fmt::Debug + 'static;

            $(
                type $field_name: $field_type + Sync + Send + 'static;
            )*
        }
    };

    // Base pattern
    (
        struct $service_name:ident : $trait:path = $dependencies:ident {
            $(
                $field_name:ident : $field_type:path = $field_attr:ident
            ),* $(,)?
        }
    ) => {
        // Generate the trait
        gen_service_impl!(@generate_trait $dependencies, $($field_name : $field_type),*);

        // Then define the struct
        pub struct $service_name<Deps: $dependencies> {
            $(
                pub $field_attr: std::sync::Arc<Deps::$field_name>,
            )*
        }

        impl<Deps: $dependencies> $service_name<Deps> {
            pub fn new($($field_attr: Deps::$field_name),*) -> Self {
                Self {
                    $($field_attr: std::sync::Arc::new($field_attr),)*
                }
            }
        }
    };
}
