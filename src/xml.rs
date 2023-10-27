use std::collections::HashMap;

use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};
//TODO implement XML parsing and marshaling
use rquickjs::{
    class::{Trace, Tracer},
    function::Opt,
    module::{Declarations, Exports, ModuleDef},
    object::Property,
    Array, Class, Ctx, Error, Function, IntoJs, Object, Result, Value,
};

use crate::util::{export_default, get_bytes, JoinToString, ObjectExt, ResultExt};

#[rquickjs::class]
struct XMLParser<'js> {
    tag_value_processor: Option<Function<'js>>,
    attribute_value_processor: Option<Function<'js>>,
    attribute_name_prefix: String,
    ignore_attributes: bool,
    text_node_name: String,
    entities: HashMap<String, String>,
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

        Ok(XMLParser {
            tag_value_processor,
            attribute_value_processor,
            entities: HashMap::new(),
            attribute_name_prefix,
            ignore_attributes,
            text_node_name,
        })
    }

    pub fn add_entity(&mut self, key: String, value: String) {
        self.entities.insert(key, value);
    }

    pub fn parse(&self, ctx: Ctx<'js>, xml: Value<'js>) -> Result<Object<'js>> {
        let bytes = get_bytes(&ctx, xml)?;

        let mut reader = Reader::from_reader(bytes.as_ref());
        reader.trim_text(true);

        let mut current_obj = Object::new(ctx.clone())?;
        let mut buf = Vec::new();
        let mut current_key = String::new();
        let mut current_value: Option<String> = None;
        let mut path: Vec<(String, Object<'js>)> = vec![];
        let mut has_attributes = false;

        loop {
            buf.clear();

            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref tag)) => {
                    current_key = Self::get_tag_name(&ctx, &reader, tag)?;

                    let obj = Object::new(ctx.clone())?;
                    self.process_attributes(&ctx, &reader, &path, tag, &obj, &mut false)?;

                    Self::process_end(&ctx, &current_obj, obj.into_value(), &current_key)?;
                }
                Ok(Event::Start(ref tag)) => {
                    current_key = Self::get_tag_name(&ctx, &reader, tag)?;
                    path.push((current_key.clone(), current_obj));

                    let obj = Object::new(ctx.clone())?;
                    current_obj = obj;

                    self.process_attributes(
                        &ctx,
                        &reader,
                        &path,
                        tag,
                        &current_obj,
                        &mut has_attributes,
                    )?;
                }
                Ok(Event::End(_)) => {
                    let (parent_tag, parent_obj) = path.pop().unwrap();
                    let value = if let Some(value) = current_value.take() {
                        value.into_js(&ctx)?
                    } else {
                        current_obj.into_value()
                    };

                    current_obj = parent_obj;

                    Self::process_end(&ctx, &current_obj, value, &parent_tag)?;

                    has_attributes = false;
                }
                Ok(Event::CData(text)) => {
                    let text = text.escape().or_throw(&ctx)?;
                    let tag_value = String::from_utf8_lossy(text.as_ref()).to_string();
                    let tag_value =
                        self.process_tag_value(&path, &current_key, tag_value, has_attributes)?;
                    if has_attributes {
                        current_obj.set(&self.text_node_name, tag_value)?;
                    } else {
                        current_value = Some(tag_value)
                    }
                }
                Ok(Event::Text(ref text)) => {
                    let tag_value = text
                        .unescape_with(|v| self.entities.get(v).map(|x| x.as_str()))
                        .or_throw(&ctx)?
                        .to_string();
                    let tag_value =
                        self.process_tag_value(&path, &current_key, tag_value, has_attributes)?;

                    if has_attributes {
                        current_obj.set(&self.text_node_name, tag_value)?;
                    } else {
                        current_value = Some(tag_value)
                    }
                }
                Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
                Ok(Event::Eof) => break,
                _ => {}
            }
        }
        Ok(current_obj)
    }
}

impl<'js> XMLParser<'js> {
    fn get_tag_name(
        ctx: &Ctx<'js>,
        reader: &Reader<&[u8]>,
        tag: &BytesStart<'_>,
    ) -> Result<String> {
        let tag = tag.name();
        let tag_name = reader.decoder().decode(tag.as_ref()).or_throw(ctx)?;

        Ok(tag_name.to_string())
    }

    fn process_end(
        ctx: &Ctx<'js>,
        current_obj: &Object<'js>,
        value: Value<'js>,
        tag: &str,
    ) -> Result<()> {
        if current_obj.contains_key(tag)? {
            let parent_value: Value = current_obj.get(tag)?;
            if !parent_value.is_array() {
                let array = Array::new(ctx.clone())?;
                array.set(0, parent_value)?;
                array.set(1, value)?;
                current_obj.set(tag, array.as_value())?;
            } else {
                let array = parent_value.as_array().or_throw(ctx)?;
                array.set(array.len(), value)?;
                current_obj.set(tag, array.as_value())?;
            }
        } else {
            current_obj.prop(
                tag,
                Property::from(value).configurable().enumerable().writable(),
            )?;
        }
        Ok(())
    }

    fn process_attributes(
        &self,
        ctx: &Ctx<'js>,
        reader: &Reader<&[u8]>,
        path: &[(String, Object<'js>)],
        tag: &BytesStart<'_>,
        current_obj: &Object<'js>,
        has_attributes: &mut bool,
    ) -> Result<()> {
        if !self.ignore_attributes {
            for attribute in tag.attributes() {
                *has_attributes = true;
                let attr = attribute.or_throw(ctx)?;

                let key_slice = attr.key.as_ref();
                let key = if !self.attribute_name_prefix.is_empty() {
                    let prefix_bytes = self.attribute_name_prefix.as_bytes();
                    let mut key_bytes = Vec::with_capacity(prefix_bytes.len() + key_slice.len());
                    key_bytes.extend_from_slice(prefix_bytes);
                    key_bytes.extend_from_slice(key_slice);

                    reader
                        .decoder()
                        .decode(&key_bytes)
                        .or_throw(ctx)?
                        .to_string()
                } else {
                    reader
                        .decoder()
                        .decode(key_slice)
                        .or_throw(ctx)?
                        .to_string()
                };

                let mut value = reader
                    .decoder()
                    .decode(attr.value.as_ref())
                    .or_throw(ctx)?
                    .to_string();

                if let Some(attribute_value_processor) = &self.attribute_value_processor {
                    let jpath: String = path.iter().join_to_string(".", |(k, _)| k);
                    if let Some(new_value) =
                        attribute_value_processor.call((key.clone(), value.clone(), jpath))?
                    {
                        value = new_value
                    }
                }
                current_obj.set(key, value)?;
            }
        }
        Ok(())
    }

    fn process_tag_value(
        &self,
        path: &[(String, Object<'js>)],
        key: &String,
        value: String,
        has_attributes: bool,
    ) -> Result<String> {
        if value.is_empty() {
            return Ok(value);
        }

        if let Some(tag_value_processor) = &self.tag_value_processor {
            let jpath: String = path.iter().join_to_string(".", |(k, _)| k);
            if let Some(new_value) =
                tag_value_processor.call((key, value.clone(), jpath, has_attributes))?
            {
                return Ok(new_value);
            }
        }
        Ok::<_, Error>(value)
    }
}

pub struct XmlModule;

impl ModuleDef for XmlModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare(stringify!(XMLParser))?;

        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            Class::<XMLParser>::define(default)?;
            Ok(())
        })?;

        Ok(())
    }
}
