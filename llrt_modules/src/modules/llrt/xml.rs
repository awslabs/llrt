// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{collections::HashMap, rc::Rc};

use llrt_utils::{
    bytes::ObjectBytes, module::export_default, object::ObjectExt, result::ResultExt,
};
use quick_xml::{
    escape::resolve_xml_entity,
    events::{BytesStart, Event},
    Reader,
};
use rquickjs::{
    class::{Trace, Tracer},
    function::Opt,
    module::{Declarations, Exports, ModuleDef},
    object::Property,
    prelude::This,
    Array, Class, Ctx, Error, Function, IntoJs, Object, Result, Value,
};

use crate::ModuleInfo;

const AMP: &str = "&amp;";
const LT: &str = "&lt;";
const GT: &str = "&gt;";
const QUOT: &str = "&quot;";
const APOS: &str = "&apos;";
const CR: &str = "&#x0D;";
const LF: &str = "&#x0A;";
const NEL: &str = "&#x85;";
const LS: &str = "&#x2028;";

#[rquickjs::class]
struct XMLParser<'js> {
    tag_value_processor: Option<Function<'js>>,
    attribute_value_processor: Option<Function<'js>>,
    attribute_name_prefix: Rc<str>,
    ignore_attributes: bool,
    text_node_name: Rc<str>,
    entities: HashMap<Rc<str>, Rc<str>>,
}

impl<'js> Trace<'js> for XMLParser<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        if let Some(tag_value_processor) = &self.tag_value_processor {
            tracer.mark(tag_value_processor)
        }
        if let Some(attribute_value_processor) = &self.attribute_value_processor {
            tracer.mark(attribute_value_processor)
        }
    }
}

struct StackObject<'js> {
    obj: Object<'js>,
    has_value: bool,
}
impl<'js> StackObject<'js> {
    fn new(ctx: Ctx<'js>) -> Result<Self> {
        Ok(Self {
            obj: Object::new(ctx)?,
            has_value: false,
        })
    }

    fn into_value(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        if self.has_value {
            return Ok(self.obj.into_value());
        }
        "".into_js(ctx)
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> XMLParser<'js> {
    #[qjs(constructor)]
    pub fn new(_ctx: Ctx<'js>, options: Opt<Object<'js>>) -> Result<Self> {
        let mut tag_value_processor = None;
        let mut attribute_value_processor = None;
        let mut attribute_name_prefix = String::from("@_");
        let mut ignore_attributes = true;
        let mut text_node_name = String::from("#text");
        if let Some(options) = options.0 {
            tag_value_processor = options.get_optional("tagValueProcessor")?;
            attribute_value_processor = options.get_optional("attributeValueProcessor")?;
            if let Some(prefix) = options.get_optional("attributeNamePrefix")? {
                attribute_name_prefix = prefix;
            }
            if let Some(attributes_ignored) = options.get_optional("ignoreAttributes")? {
                ignore_attributes = attributes_ignored
            }
            if let Some(name) = options.get_optional("textNodeName")? {
                text_node_name = name
            }
        }

        let entities = HashMap::new();

        Ok(XMLParser {
            tag_value_processor,
            attribute_value_processor,
            entities,
            attribute_name_prefix: attribute_name_prefix.into(),
            ignore_attributes,
            text_node_name: text_node_name.into(),
        })
    }

    pub fn add_entity(&mut self, key: String, value: String) {
        self.entities.insert(key.into(), value.into());
    }

    pub fn parse(&self, ctx: Ctx<'js>, xml: Value<'js>) -> Result<Object<'js>> {
        let bytes = ObjectBytes::from(&ctx, &xml)?;
        let bytes = bytes.as_bytes();
        let mut reader = Reader::from_reader(bytes);
        let config = reader.config_mut();
        config.trim_text(true);

        let mut current_obj = StackObject::new(ctx.clone())?;
        current_obj.has_value = true;
        let mut buf = Vec::new();
        let mut current_key: Rc<str> = "".into();
        let mut current_value: Option<Rc<str>> = None;
        let mut path: Vec<(Rc<str>, StackObject<'js>)> = vec![];
        let mut has_attributes = false;

        loop {
            buf.clear();

            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref tag)) => {
                    current_key = Self::get_tag_name(&ctx, &reader, tag)?;

                    let mut obj = StackObject::new(ctx.clone())?;
                    self.process_attributes(&ctx, &reader, &path, tag, &mut obj, &mut false)?;
                    current_obj.has_value = true;

                    Self::process_end(&ctx, &current_obj, obj.into_value(&ctx)?, &current_key)?;
                },
                Ok(Event::Start(ref tag)) => {
                    has_attributes = false;
                    current_key = Self::get_tag_name(&ctx, &reader, tag)?;
                    path.push((current_key.clone(), current_obj));

                    let obj = StackObject::new(ctx.clone())?;
                    current_obj = obj;

                    self.process_attributes(
                        &ctx,
                        &reader,
                        &path,
                        tag,
                        &mut current_obj,
                        &mut has_attributes,
                    )?;
                },
                Ok(Event::End(_)) => {
                    let (parent_tag, mut parent_obj) = path.pop().unwrap();
                    parent_obj.has_value = true;
                    let value = if let Some(value) = current_value.take() {
                        value.into_js(&ctx)?
                    } else {
                        current_obj.into_value(&ctx)?
                    };

                    current_obj = parent_obj;

                    Self::process_end(&ctx, &current_obj, value, &parent_tag)?;
                },
                Ok(Event::CData(text)) => {
                    let text = text.escape().or_throw(&ctx)?;
                    let tag_value = String::from_utf8_lossy(text.as_ref());
                    let tag_value = tag_value.as_ref();
                    let tag_value =
                        self.process_tag_value(&path, &current_key, tag_value, has_attributes)?;
                    if has_attributes {
                        current_obj.has_value = true;
                        current_obj
                            .obj
                            .set(self.text_node_name.as_ref(), tag_value.as_ref())?;
                    } else {
                        current_value = Some(tag_value)
                    }
                },
                Ok(Event::Text(ref text)) => {
                    let tag_value = text
                        .unescape_with(|v| {
                            resolve_xml_entity(v)
                                .or_else(|| self.entities.get(v).map(|x| x.as_ref()))
                        })
                        .or_throw(&ctx)?;
                    let tag_value = tag_value.as_ref();
                    let tag_value =
                        self.process_tag_value(&path, &current_key, tag_value, has_attributes)?;

                    if has_attributes {
                        current_obj.has_value = true;
                        current_obj
                            .obj
                            .set(self.text_node_name.as_ref(), tag_value.as_ref())?;
                    } else {
                        current_value = Some(tag_value)
                    }
                },
                Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
                Ok(Event::Eof) => break,
                _ => {},
            }
        }
        Ok(current_obj.obj)
    }
}

impl<'js> XMLParser<'js> {
    fn get_tag_name(
        ctx: &Ctx<'js>,
        reader: &Reader<&[u8]>,
        tag: &BytesStart<'_>,
    ) -> Result<Rc<str>> {
        let tag = tag.name();
        let tag_name = reader.decoder().decode(tag.as_ref()).or_throw(ctx)?;
        Ok(tag_name.as_ref().into())
    }

    fn process_end(
        ctx: &Ctx<'js>,
        current_obj: &StackObject<'js>,
        value: Value<'js>,
        tag: &str,
    ) -> Result<()> {
        if current_obj.obj.contains_key(tag)? {
            let parent_value: Value = current_obj.obj.get(tag)?;
            if !parent_value.is_array() {
                let array = Array::new(ctx.clone())?;
                array.set(0, parent_value)?;
                array.set(1, value)?;
                current_obj.obj.set(tag, array.as_value())?;
            } else {
                let array = parent_value.as_array().or_throw(ctx)?;
                array.set(array.len(), value)?;
                current_obj.obj.set(tag, array.as_value())?;
            }
        } else {
            current_obj.obj.prop(
                tag,
                Property::from(value).configurable().enumerable().writable(),
            )?;
        }
        Ok(())
    }

    fn set_attribute(
        &self,
        stack_object: &mut StackObject<'js>,
        path: &[&str],
        key: &str,
        value: &str,
    ) -> Result<()> {
        if let Some(attribute_value_processor) = &self.attribute_value_processor {
            let jpath = path.join(".");
            let jpath = jpath.as_str();
            if let Some(new_value) =
                attribute_value_processor.call::<_, Option<String>>((key, value, jpath))?
            {
                stack_object.obj.set(key, new_value)?;
                return Ok(());
            }
        }
        stack_object.obj.set(key, value)?;
        Ok(())
    }

    fn process_attributes(
        &self,
        ctx: &Ctx<'js>,
        reader: &Reader<&[u8]>,
        path: &[(Rc<str>, StackObject<'js>)],
        tag: &BytesStart<'_>,
        stack_object: &mut StackObject<'js>,
        has_attributes: &mut bool,
    ) -> Result<()> {
        if !self.ignore_attributes {
            let jpath_str = path.iter().map(|(k, _)| k.as_ref()).collect::<Vec<_>>();
            for attribute in tag.attributes() {
                stack_object.has_value = true;
                *has_attributes = true;
                let attr = attribute.or_throw(ctx)?;

                let value = reader.decoder().decode(attr.value.as_ref()).or_throw(ctx)?;
                let value = value.as_ref();

                let key_slice = attr.key.as_ref();
                if !self.attribute_name_prefix.is_empty() {
                    let prefix_bytes = self.attribute_name_prefix.as_bytes();
                    let mut key_bytes = Vec::with_capacity(prefix_bytes.len() + key_slice.len());
                    key_bytes.extend_from_slice(prefix_bytes);
                    key_bytes.extend_from_slice(key_slice);

                    let key = reader.decoder().decode(&key_bytes).or_throw(ctx)?;
                    self.set_attribute(stack_object, &jpath_str, key.as_ref(), value)?;
                } else {
                    let key = reader.decoder().decode(key_slice).or_throw(ctx)?;
                    self.set_attribute(stack_object, &jpath_str, key.as_ref(), value)?;
                };
            }
        }
        Ok(())
    }

    fn process_tag_value(
        &self,
        path: &[(Rc<str>, StackObject<'js>)],
        key: &str,
        value: &str,
        has_attributes: bool,
    ) -> Result<Rc<str>> {
        if value.is_empty() {
            return Ok(value.into());
        }

        if let Some(tag_value_processor) = &self.tag_value_processor {
            let jpath = path
                .iter()
                .map(|(k, _)| k.as_ref())
                .collect::<Vec<_>>()
                .join(".");

            if let Some(new_value) = tag_value_processor.call::<_, Option<String>>((
                key,
                value,
                jpath,
                has_attributes,
            ))? {
                return Ok(new_value.into());
            }
        }
        Ok::<_, Error>(value.into())
    }
}

#[derive(Debug, Clone)]
#[rquickjs::class]
struct XmlText {
    value: Rc<str>,
}

impl<'js> Trace<'js> for XmlText {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

#[rquickjs::methods(rename_all = "camelCase")]
impl XmlText {
    #[qjs(constructor)]
    fn new(value: String) -> Self {
        let mut escaped = String::with_capacity(value.len());
        escape_element(&mut escaped, &value);
        XmlText {
            value: escaped.into(),
        }
    }

    fn to_string(&self) -> &str {
        self.value.as_ref()
    }
}

#[derive(Debug, Clone)]
#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
struct XmlNode<'js> {
    #[qjs(skip_trace)]
    name: String,
    //child and attributes are always set to avoid branch checks when adding/removing values
    children: Vec<Value<'js>>,
    #[qjs(skip_trace)]
    //vec iteration is faster since we rarely have more than 10 attrs and we want to retain insertion order
    attributes: Vec<(String, String)>,
}

enum NodeStackEntry<'js> {
    Node(Class<'js, XmlNode<'js>>),
    End(String),
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> XmlNode<'js> {
    #[qjs(constructor)]
    fn new(name: String, children: Opt<Vec<Value<'js>>>) -> Result<Self> {
        let node = XmlNode {
            name,
            attributes: Vec::new(),
            children: children.0.unwrap_or_default(),
        };

        Ok(node)
    }

    #[qjs(static)]
    fn of(
        ctx: Ctx<'js>,
        name: String,
        child_text: Opt<String>,
        with_name: Opt<String>,
    ) -> Result<Value<'js>> {
        let mut node = XmlNode {
            name,
            children: Vec::new(),
            attributes: Vec::new(),
        };

        if let Some(text) = child_text.0 {
            let xml_text = Class::instance(ctx.clone(), XmlText::new(text))?;
            node.children.push(xml_text.into_value());
        }

        if let Some(new_name) = with_name.0 {
            node.name = new_name;
        }

        node.into_js(&ctx)
    }

    fn with_name(this: This<Class<'js, Self>>, name: String) -> Class<'js, Self> {
        this.borrow_mut().name = name;
        this.0
    }

    fn add_attribute(
        this: This<Class<'js, Self>>,
        name: String,
        value: String,
    ) -> Class<'js, Self> {
        let this2 = this.clone();
        let mut borrow = this2.borrow_mut();
        if let Some(pos) = borrow.attributes.iter().position(|(a, _)| a == &name) {
            borrow.attributes[pos] = (name, value);
        } else {
            borrow.attributes.push((name, value));
        }
        this.0
    }

    fn add_child_node(this: This<Class<'js, Self>>, value: Value<'js>) -> Result<Class<'js, Self>> {
        let this2 = this.clone();
        this2.borrow_mut().children.push(value);
        Ok(this.0)
    }

    fn remove_attribute(this: This<Class<'js, Self>>, name: String) -> Class<'js, Self> {
        let this2 = this.clone();
        let mut borrow = this2.borrow_mut();
        if let Some(pos) = borrow.attributes.iter().position(|(a, _)| a == &name) {
            borrow.attributes.remove(pos);
        }
        this.0
    }

    fn to_string(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<String> {
        let class = this.0;
        let mut xml_text = String::with_capacity(8);

        let mut stack = vec![NodeStackEntry::Node(class)];

        while let Some(node) = stack.pop() {
            match node {
                NodeStackEntry::Node(node) => {
                    let borrow = node.borrow();
                    xml_text.push('<');
                    xml_text.push_str(&borrow.name);

                    for (attribute_name, attribute) in &borrow.attributes {
                        xml_text.push(' ');
                        xml_text.push_str(attribute_name);
                        xml_text.push_str("=\"");
                        escape_attribute(&mut xml_text, attribute);
                        xml_text.push('"');
                    }

                    let has_children = !borrow.children.is_empty();
                    if has_children {
                        stack.push(NodeStackEntry::End(borrow.name.clone()));
                        xml_text.push('>');

                        // Add children to the stack in reverse order (to maintain original order)
                        for child in borrow.children.iter().rev() {
                            if let Some(obj) = child.as_object() {
                                if let Some(node) = Class::<Self>::from_object(obj) {
                                    stack.push(NodeStackEntry::Node(node))
                                } else if let Some(text) = Class::<XmlText>::from_object(obj) {
                                    xml_text.push_str(&text.borrow().value);
                                } else {
                                    let to_string_fn = obj.get::<_, Function>("toString")?;
                                    let string_value: String = to_string_fn.call(())?;
                                    xml_text.push_str(&string_value);
                                }
                            } else {
                                let string_value = child
                                    .clone()
                                    .try_into_string()
                                    .map_err(|err| format!("Unable to convert {:?} to string", err))
                                    .or_throw(&ctx)?
                                    .to_string()?;
                                xml_text.push_str(&string_value);
                            }
                        }
                    } else {
                        xml_text.push_str("/>");
                    }
                    drop(borrow);
                },
                NodeStackEntry::End(name) => {
                    xml_text.push_str("</");
                    xml_text.push_str(&name);
                    xml_text.push('>');
                },
            }
        }

        Ok(xml_text)
    }
}

fn escape_attribute(text: &mut String, value: &str) {
    text.reserve(value.len());
    for c in value.chars() {
        match c {
            '&' => text.push_str(AMP),
            '<' => text.push_str(LT),
            '>' => text.push_str(GT),
            '"' => text.push_str(QUOT),
            _ => text.push(c),
        }
    }
}

fn escape_element(text: &mut String, value: &str) {
    text.reserve(value.len());
    for c in value.chars() {
        match c {
            '&' => text.push_str(AMP),
            '<' => text.push_str(LT),
            '>' => text.push_str(GT),
            '\'' => text.push_str(APOS),
            '"' => text.push_str(QUOT),
            '\r' => text.push_str(CR),
            '\n' => text.push_str(LF),
            '\u{0085}' => text.push_str(NEL),
            '\u{2028}' => text.push_str(LS),
            _ => text.push(c),
        }
    }
}

pub struct LlrtXmlModule;

impl ModuleDef for LlrtXmlModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare(stringify!(XMLParser))?;
        declare.declare(stringify!(XmlText))?;
        declare.declare(stringify!(XmlNode))?;

        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            Class::<XMLParser>::define(default)?;
            Class::<XmlText>::define(default)?;
            Class::<XmlNode>::define(default)?;
            Ok(())
        })?;

        Ok(())
    }
}

impl From<LlrtXmlModule> for ModuleInfo<LlrtXmlModule> {
    fn from(val: LlrtXmlModule) -> Self {
        ModuleInfo {
            name: "llrt:xml",
            module: val,
        }
    }
}
