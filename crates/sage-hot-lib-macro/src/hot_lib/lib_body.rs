use syn::{Attribute, Ident, Item, Visibility};

pub(crate) struct HotLibrary {
    pub(crate) vis: Visibility,
    pub(crate) ident: Ident,
    pub(crate) items: Vec<Item>,
    pub(crate) attributes: Vec<Attribute>,
    pub(crate) hot_lib_args: Option<super::HotLibAttribute>,
}
