
/// Macro to define an enum of events with optional derives, automatic `From` impls, and `apply` dispatcher
#[macro_export]
macro_rules! pub_events_enum {
    (
        $(#[$outer:meta])*
        pub enum $name:ident {
            $($variant:ident),* $(,)?
        }
    ) => {
        // Enum definition with variants holding the struct
        $(#[$outer])*
        pub enum $name {
            $(
                $variant($variant),
            )*
        }

        // From impls for automatic conversion from struct to enum variant
        $(
            impl From<$variant> for $name {
                fn from(e: $variant) -> Self {
                    $name::$variant(e)
                }
            }
        )*

        // Auto-generated DomainEvent trait implementation
        impl<A: Aggregate> DomainEvent<A> for $name 
        where
        $(
            $variant: DomainEvent<A>,
        )* {
            fn aggregate_id(&self) -> A::Id {
                match self {
                    $(
                        $name::$variant(e) => e.aggregate_id(),
                    )*
                }
            }
            
            fn apply(&self, state: &mut A)-> Uuid {
                match self {
                    $(
                        $name::$variant(e) => e.apply(state),
                    )*
                }
            }
        }
    };
}
