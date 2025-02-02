use openapiv3::ReferenceOr;

pub trait ReferenceOrExt<T> {
    fn into_reference(self) -> Option<String>;
}

impl<T> ReferenceOrExt<T> for ReferenceOr<T> {
    fn into_reference(self) -> Option<String> {
        match self {
            ReferenceOr::Reference { reference } => Some(reference),
            ReferenceOr::Item(_) => None,
        }
    }
}
