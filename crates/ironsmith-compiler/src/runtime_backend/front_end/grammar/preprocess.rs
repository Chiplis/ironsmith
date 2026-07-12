#[path = "preprocess/borrow_expansion.rs"]
mod borrow_expansion;
#[path = "preprocess/borrow_shapes.rs"]
mod borrow_shapes;
#[path = "preprocess/document_shapes.rs"]
mod document_shapes;
#[path = "preprocess/line_shapes.rs"]
mod line_shapes;
#[path = "preprocess/name_shapes.rs"]
mod name_shapes;
#[path = "preprocess/vote_shapes.rs"]
mod vote_shapes;

pub(crate) use borrow_expansion::*;
pub(crate) use borrow_shapes::*;
pub(crate) use document_shapes::*;
pub(crate) use line_shapes::*;
pub(crate) use name_shapes::*;
pub(crate) use vote_shapes::*;
