#![allow(unused)]

use std::{
	rc::Rc,
	collections::HashMap, borrow::BorrowMut,
};

use chumsky::primitive::todo;

use crate::nbt::{
	MapType,
	tag::*,
	tagtype::*,
};

pub struct ValueEditorArgs<'a, T> {
	// ui: &mut egui::Ui,
	node: &'a mut Editable<T>,
	value: Rc<T>
}

pub trait ValueEditor<T> {
	// fn edit_value(&mut self, ui: &mut egui::Ui, node: &mut Editable<T>, value: Rc<T>) -> egui::Response;
}

pub struct EditWidget<T> {
	value: Rc<T>,
	editor: Box<dyn ValueEditor<T>>,
}

impl<T> EditWidget<T> {
	pub fn new(value: Rc<T>, editor: Box<dyn ValueEditor<T>>) -> Self {
		Self {
			value,
			editor,
		}
	}

	pub fn value(&self) -> Rc<T> {
		self.value.clone()
	}
}

pub enum Editable<T> {
	Value(Rc<T>),
	Editor(Box<EditWidget<T>>),
}

impl<T> AsRef<T> for Editable<T> {
	fn as_ref(&self) -> &T {
		match self {
			Editable::Value(value) => value.as_ref(),
			Editable::Editor(widget) => widget.value.as_ref(),
		}
	}
}

impl<T> Editable<T> {

	pub fn new(value: T) -> Self {
		Self::Value(Rc::new(value))
	}

	pub fn value(&self) -> Rc<T> {
		match self {
			Editable::Value(obj) => obj.clone(),
			Editable::Editor(widget) => widget.value(),
		}
	}

	pub fn editing(&self) -> bool {
		matches!(self, Editable::Editor(_))
	}

	pub fn begin_edit(&mut self, editor: Box<dyn ValueEditor<T>>) {
		match self {
			Editable::Value(obj) => {
				*self = Self::Editor(Box::new(
					EditWidget::new(obj.clone(), editor)
				));
			},
			Editable::Editor(widget) => {
				*self = Self::Editor(Box::new(
					EditWidget::new(widget.value.clone(), editor)
				));
			},
		}
	}

	pub fn end_edit(&mut self) {
		if let Editable::Editor(widget) = self {
			*self = Self::Value(widget.value.clone());
		}
	}
}

pub type EditableMap = MapType<Editable<EditableTag>>;

// DECIDE: Do I also want to include a widget slot?
#[repr(isize)]
pub enum EditableTag {
	Byte(Editable<Byte>) = 1,
	Short(Editable<Short>) = 2,
	Int(Editable<Int>) = 3,
	Long(Editable<Long>) = 4,
	Float(Editable<Float>) = 5,
	Double(Editable<Double>) = 6,
	ByteArray(Editable<ByteArray>) = 7,
	String(Editable<String>) = 8,
	List(Editable<EditableListTag>) = 9,
	Compound(Editable<EditableMap>) = 10,
	IntArray(Editable<IntArray>) = 11,
	LongArray(Editable<LongArray>) = 12,
}

type EditableVec<T> = Editable<Vec<Editable<T>>>;

#[repr(isize)]
pub enum EditableListTag {
	Empty = 0,
	Byte(EditableVec<Byte>) = 1,
	Short(EditableVec<Short>) = 2,
	Int(EditableVec<Int>) = 3,
	Long(EditableVec<Long>) = 4,
	Float(EditableVec<Float>) = 5,
	Double(EditableVec<Double>) = 6,
	ByteArray(EditableVec<ByteArray>) = 7,
	String(EditableVec<String>) = 8,
	List(EditableVec<EditableListTag>) = 9,
	Compound(EditableVec<EditableMap>) = 10,
	IntArray(EditableVec<IntArray>) = 11,
	LongArray(EditableVec<LongArray>) = 12,
}

impl EditableTag {
	pub fn id(&self) -> TagID {
		match self {
			EditableTag::Byte(_) => TagID::Byte,
			EditableTag::Short(_) => TagID::Short,
			EditableTag::Int(_) => TagID::Int,
			EditableTag::Long(_) => TagID::Long,
			EditableTag::Float(_) => TagID::Float,
			EditableTag::Double(_) => TagID::Double,
			EditableTag::ByteArray(_) => TagID::ByteArray,
			EditableTag::String(_) => TagID::String,
			EditableTag::List(_) => TagID::List,
			EditableTag::Compound(_) => TagID::Compound,
			EditableTag::IntArray(_) => TagID::IntArray,
			EditableTag::LongArray(_) => TagID::LongArray
		}
	}
}

impl EditableListTag {
	pub fn id(&self) -> TagID {
		match self {
			EditableListTag::Empty => TagID::Byte,
			EditableListTag::Byte(_) => TagID::Byte,
			EditableListTag::Short(_) => TagID::Short,
			EditableListTag::Int(_) => TagID::Int,
			EditableListTag::Long(_) => TagID::Long,
			EditableListTag::Float(_) => TagID::Float,
			EditableListTag::Double(_) => TagID::Double,
			EditableListTag::ByteArray(_) => TagID::ByteArray,
			EditableListTag::String(_) => TagID::String,
			EditableListTag::List(_) => TagID::List,
			EditableListTag::Compound(_) => TagID::Compound,
			EditableListTag::IntArray(_) => TagID::IntArray,
			EditableListTag::LongArray(_) => TagID::LongArray
		}
	}
}

fn map_to_editable(map: &MapType<Tag>) -> EditableMap {
	let mut result = EditableMap::new();
	map.iter().for_each(|(key, tag)| {
		result.insert(key.to_owned(), Editable::new(EditableTag::from(tag.clone())));
	});
	result
}

fn make_editable_vec<T: Clone>(items: Vec<T>) -> Editable<Vec<Editable<T>>> {
	Editable::new(
		items.iter().map(|value| value.into()).collect()
	)
}

impl From<Vec<ListTag>> for EditableVec<EditableListTag> {
	fn from(value: Vec<ListTag>) -> Self {
		Editable::new(
			value.iter()
				.map(EditableListTag::from)
				.map(Editable::new)
				.collect()
		)
	}
}

impl From<ListTag> for Editable<EditableListTag> {
    fn from(value: ListTag) -> Self {
        Editable::new(value.into())
    }
}

impl From<&ListTag> for Editable<EditableListTag> {
    fn from(value: &ListTag) -> Self {
        Editable::new(value.into())
    }
}

impl From<MapType<Tag>> for Editable<EditableMap> {
    fn from(value: MapType<Tag>) -> Self {
        Editable::new({
			let mut result = EditableMap::new();
			value.iter().for_each(|(key, tag)| {
				result.insert(
					key.to_owned(),
					Editable::new(EditableTag::from(tag.clone()))
				);
			});
			result
		})
    }
}

impl From<&MapType<Tag>> for Editable<EditableMap> {
    fn from(value: &MapType<Tag>) -> Self {
        Editable::new({
			let mut result = EditableMap::new();
			value.iter().for_each(|(key, tag)| {
				result.insert(
					key.to_owned(),
					Editable::new(EditableTag::from(tag.clone()))
				);
			});
			result
		})
    }
}

impl From<Vec<MapType<Tag>>> for EditableVec<EditableMap> {
    fn from(value: Vec<MapType<Tag>>) -> Self {
        Editable::new(
			value.iter()
				.map(map_to_editable)
				.map(Editable::new)
				.collect()
		)
    }
}

impl From<&Vec<MapType<Tag>>> for EditableVec<EditableMap> {
    fn from(value: &Vec<MapType<Tag>>) -> Self {
        Editable::new(
			value.iter()
				.map(map_to_editable)
				.map(Editable::new)
				.collect()
		)
    }
}

impl From<Tag> for EditableTag {
    fn from(value: Tag) -> Self {
        match value {
            Tag::Byte(value) => EditableTag::Byte(value.into()),
            Tag::Short(value) => EditableTag::Short(value.into()),
            Tag::Int(value) => EditableTag::Int(value.into()),
            Tag::Long(value) => EditableTag::Long(value.into()),
            Tag::Float(value) => EditableTag::Float(value.into()),
            Tag::Double(value) => EditableTag::Double(value.into()),
            Tag::ByteArray(value) => EditableTag::ByteArray(value.into()),
            Tag::String(value) => EditableTag::String(value.into()),
            Tag::List(value) => EditableTag::List(value.into()),
            Tag::Compound(value) => EditableTag::Compound(value.into()),
            Tag::IntArray(value) => EditableTag::IntArray(value.into()),
            Tag::LongArray(value) => EditableTag::LongArray(value.into()),
        }
    }
}

impl From<&Tag> for EditableTag {
    fn from(value: &Tag) -> Self {
        match value {
            Tag::Byte(value) => EditableTag::Byte(value.into()),
            Tag::Short(value) => EditableTag::Short(value.into()),
            Tag::Int(value) => EditableTag::Int(value.into()),
            Tag::Long(value) => EditableTag::Long(value.into()),
            Tag::Float(value) => EditableTag::Float(value.into()),
            Tag::Double(value) => EditableTag::Double(value.into()),
            Tag::ByteArray(value) => EditableTag::ByteArray(value.into()),
            Tag::String(value) => EditableTag::String(value.into()),
            Tag::List(value) => EditableTag::List(value.into()),
            Tag::Compound(value) => EditableTag::Compound(value.into()),
            Tag::IntArray(value) => EditableTag::IntArray(value.into()),
            Tag::LongArray(value) => EditableTag::LongArray(value.into()),
        }
    }
}

impl From<ListTag> for EditableListTag {
	fn from(value: ListTag) -> Self {
		match value {
			ListTag::Empty => EditableListTag::Empty,
			ListTag::Byte(list) => EditableListTag::Byte(list.into()),
			ListTag::Short(list) => EditableListTag::Short(list.into()),
			ListTag::Int(list) => EditableListTag::Int(list.into()),
			ListTag::Long(list) => EditableListTag::Long(list.into()),
			ListTag::Float(list) => EditableListTag::Float(list.into()),
			ListTag::Double(list) => EditableListTag::Double(list.into()),
			ListTag::ByteArray(list) => EditableListTag::ByteArray(list.into()),
			ListTag::String(list) => EditableListTag::String(list.into()),
			ListTag::List(list) => EditableListTag::List(list.into()),
			ListTag::Compound(list) => EditableListTag::Compound(list.into()),
			ListTag::IntArray(list) => EditableListTag::IntArray(list.into()),
			ListTag::LongArray(list) => EditableListTag::LongArray(list.into()),
		}
	}
}

impl From<&ListTag> for EditableListTag {
    fn from(value: &ListTag) -> Self {
        match value {
			ListTag::Empty => EditableListTag::Empty,
			ListTag::Byte(list) => EditableListTag::Byte(list.into()),
			ListTag::Short(list) => EditableListTag::Short(list.into()),
			ListTag::Int(list) => EditableListTag::Int(list.into()),
			ListTag::Long(list) => EditableListTag::Long(list.into()),
			ListTag::Float(list) => EditableListTag::Float(list.into()),
			ListTag::Double(list) => EditableListTag::Double(list.into()),
			ListTag::ByteArray(list) => EditableListTag::ByteArray(list.into()),
			ListTag::String(list) => EditableListTag::String(list.into()),
			ListTag::List(list) => EditableListTag::List(list.into()),
			ListTag::Compound(list) => EditableListTag::Compound(list.into()),
			ListTag::IntArray(list) => EditableListTag::IntArray(list.into()),
			ListTag::LongArray(list) => EditableListTag::LongArray(list.into()),
		}
    }
}

impl<T> From<T> for Editable<T> {
    fn from(value: T) -> Self {
        Editable::new(value)
    }
}

impl<T: Clone> From<&T> for Editable<T> {
    fn from(value: &T) -> Self {
        Editable::new(value.clone())
    }
}

impl<T: Clone> From<Vec<T>> for EditableVec<T> {
    fn from(value: Vec<T>) -> Self {
        Editable::from(
			value.iter()
				.map(Editable::from)
				.collect::<Vec<Editable<T>>>()
		)
    }
}

impl<T: Clone> From<&Vec<T>> for EditableVec<T> {
    fn from(value: &Vec<T>) -> Self {
        Editable::from(
			value.iter()
				.map(Editable::from)
				.collect::<Vec<Editable<T>>>()
		)
    }
}

impl From<&Vec<ListTag>> for EditableVec<EditableListTag> {
	fn from(value: &Vec<ListTag>) -> Self {
		Editable::new(value.iter()
			.map(|item| Editable::new(EditableListTag::from(item)))
			.collect::<Vec<Editable<EditableListTag>>>())
    }
}