use std::{
    borrow::Cow,
    fmt,
    fmt::{Display, Formatter},
    io,
    io::Read,
};

use main_error::MainError;
use tl::{Node, Parser, ParserOptions};

fn main() -> Result<(), MainError> {
    let mut raw = String::new();
    io::stdin().read_to_string(&mut raw)?;
    let dom = tl::parse(&raw, ParserOptions::default())?;
    for node in dom.children() {
        let Some(node) = build_kotlin_node(node.get(dom.parser()).unwrap(), dom.parser()) else {
            continue;
        };
        let dsl_node = KotlinDslIndentedNode {
            indent: 0,
            node: &node,
        };
        println!("{dsl_node}");
    }
    Ok(())
}

#[allow(clippy::type_complexity)]
fn build_kotlin_node<'a>(node: &'a Node, parser: &'a Parser) -> Option<KotlinDslNode> {
    match node {
        Node::Tag(tag) => {
            let (classes, attributes): (
                Vec<(Cow<'a, str>, Cow<'a, str>)>,
                Vec<(Cow<'a, str>, Cow<'a, str>)>,
            ) = tag
                .attributes()
                .iter()
                .filter_map(|(key, value)| Some((key, value?)))
                .partition(|(key, _)| key.as_ref() == "class");
            let children = tag
                .children()
                .top()
                .iter()
                .filter_map(|child| build_kotlin_node(child.get(parser)?, parser))
                .collect();
            Some(KotlinDslNode::Tag {
                name: tag.name().as_utf8_str().as_ref().to_owned(),
                classes: classes
                    .into_iter()
                    .next()
                    .map(|(_, v)| v.as_ref().to_owned()),
                attributes: attributes
                    .into_iter()
                    .map(|(k, v)| (k.as_ref().to_owned(), v.as_ref().to_owned()))
                    .collect(),
                children,
            })
        }
        Node::Raw(raw) => {
            let string = raw.as_utf8_str().as_ref().trim().to_owned();
            if string.is_empty() {
                return None;
            }
            Some(KotlinDslNode::String(string))
        }
        Node::Comment(comment) => {
            let string = comment
                .as_utf8_str()
                .as_ref()
                .trim()
                .strip_prefix("<!--")
                .and_then(|s| s.strip_suffix("-->"))
                .map(|s| s.trim().to_owned())
                .unwrap_or_else(|| comment.as_utf8_str().trim().to_owned());
            if string.is_empty() {
                return None;
            }
            Some(KotlinDslNode::Comment(string))
        }
    }
}

struct KotlinDslIndentedNode<'a> {
    indent: u16,
    node: &'a KotlinDslNode,
}

enum KotlinDslNode {
    Tag {
        name: String,
        classes: Option<String>,
        attributes: Vec<(String, String)>,
        children: Vec<KotlinDslNode>,
    },
    String(String),
    Comment(String),
}

impl<'a> Display for KotlinDslIndentedNode<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write_indent(f, self.indent)?;
        match &self.node {
            KotlinDslNode::Tag {
                name,
                classes,
                attributes,
                children,
            } => {
                write!(f, "{name}")?;
                if let Some(classes) = classes {
                    write!(f, "(classes=\"{classes}\")")?;
                }
                if attributes.is_empty() && children.is_empty() {
                    if classes.is_none() {
                        write!(f, "()")?;
                    }
                    {
                        write!(f, "")?;
                    }
                    return Ok(());
                }
                writeln!(f, " {{")?;
                for (key, value) in attributes.iter() {
                    write_indent(f, self.indent + 1)?;
                    writeln!(f, "{key}=\"{value}\"")?;
                }
                for child in children.iter().map(|node| KotlinDslIndentedNode {
                    indent: self.indent + 1,
                    node,
                }) {
                    writeln!(f, "{child}")?;
                }
                write_indent(f, self.indent)?;
                write!(f, "}}")?;
                Ok(())
            }
            KotlinDslNode::String(s) => write!(f, "+\"{s}\""),
            KotlinDslNode::Comment(s) => write!(f, "// {s}"),
        }
    }
}

fn write_indent(buffer: &mut impl fmt::Write, indent_level: u16) -> Result<(), fmt::Error> {
    for _ in 0..indent_level {
        write!(buffer, " ")?;
    }
    Ok(())
}
