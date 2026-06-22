//! Lua-like `modinfo.lua` parser and mod config shaping.

use serde_json::{Map, Value, json};

const MAX_LUA_TABLE_DEPTH: usize = 64;
pub(super) fn mod_config_from_lua_source(script: &str, lang: &str, mod_id: &str) -> String {
    let body = strip_lua_comments(script);
    let mut object = Map::new();
    let mut parser = LuaLikeParser::new(&body, lang, mod_id);
    while parser.skip_ws() {
        let Some(key) = parser.parse_identifier() else {
            parser.advance_char();
            continue;
        };
        parser.skip_ws();
        if !parser.consume_char('=') {
            continue;
        }
        parser.skip_ws();
        if let Some(value) = parser.parse_value() {
            if !parser.take_dropped_over_nested() {
                object.insert(key, value);
            }
        } else {
            parser.take_dropped_over_nested();
        }
    }
    Value::Object(object).to_string()
}

fn strip_lua_comments(script: &str) -> String {
    script
        .lines()
        .map(|line| line.split_once("--").map_or(line, |(before, _)| before))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn parse_mod_config(raw: &str) -> Value {
    if raw.trim().is_empty() {
        return Value::Null;
    }
    serde_json::from_str(raw).unwrap_or(Value::Null)
}
struct LuaLikeParser<'a> {
    input: &'a str,
    pos: usize,
    lang: &'a str,
    folder_name: String,
    dropped_over_nested: bool,
}

impl<'a> LuaLikeParser<'a> {
    fn new(input: &'a str, lang: &'a str, mod_id: &str) -> Self {
        Self {
            input,
            pos: 0,
            lang,
            folder_name: format!("workshop-{mod_id}"),
            dropped_over_nested: false,
        }
    }

    fn skip_ws(&mut self) -> bool {
        while let Some(character) = self.peek_char() {
            if !character.is_whitespace() {
                break;
            }
            self.advance_char();
        }
        self.pos < self.input.len()
    }

    fn parse_identifier(&mut self) -> Option<String> {
        let mut chars = self.input[self.pos..].char_indices();
        let (_, first) = chars.next()?;
        if !(first == '_' || first.is_ascii_alphabetic()) {
            return None;
        }
        let start = self.pos;
        self.pos += first.len_utf8();
        while let Some(character) = self.peek_char() {
            if character == '_' || character.is_ascii_alphanumeric() {
                self.advance_char();
            } else {
                break;
            }
        }
        Some(self.input[start..self.pos].to_owned())
    }

    fn parse_value(&mut self) -> Option<Value> {
        self.parse_value_at_depth(0)
    }

    fn parse_value_at_depth(&mut self, depth: usize) -> Option<Value> {
        self.skip_ws();
        match self.peek_char()? {
            '"' | '\'' => self.parse_string().map(Value::String),
            '{' => self.parse_table(depth + 1),
            '-' | '0'..='9' => self.parse_number().map(|number| json!(number)),
            _ => self.parse_keyword_or_call(depth),
        }
    }

    fn parse_string(&mut self) -> Option<String> {
        let quote = self.advance_char()?;
        let mut value = String::new();
        let mut escaped = false;
        while let Some(character) = self.advance_char() {
            if escaped {
                value.push(match character {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    other => other,
                });
                escaped = false;
                continue;
            }
            if character == '\\' {
                escaped = true;
                continue;
            }
            if character == quote {
                return Some(value);
            }
            value.push(character);
        }
        None
    }

    fn parse_number(&mut self) -> Option<f64> {
        let start = self.pos;
        if self.peek_char() == Some('-') {
            self.advance_char();
        }
        while self.peek_char().is_some_and(|character| {
            character.is_ascii_digit() || matches!(character, '.' | 'e' | 'E' | '+' | '-')
        }) {
            self.advance_char();
        }
        self.input[start..self.pos].parse().ok()
    }

    fn parse_keyword_or_call(&mut self, depth: usize) -> Option<Value> {
        let identifier = self.parse_identifier()?;
        match identifier.as_str() {
            "true" => Some(Value::Bool(true)),
            "false" => Some(Value::Bool(false)),
            "nil" => Some(Value::Null),
            "locale" => Some(Value::String(self.lang.to_owned())),
            "folder_name" => Some(Value::String(self.folder_name.clone())),
            "ChooseTranslationTable" => {
                self.skip_ws();
                if !self.consume_char('(') {
                    return Some(Value::String(identifier));
                }
                let value = self.parse_value_at_depth(depth).unwrap_or(Value::Null);
                self.skip_ws();
                self.consume_char(')');
                Some(select_translation_value(value, self.lang))
            }
            _ => Some(Value::String(identifier)),
        }
    }

    fn parse_table(&mut self, depth: usize) -> Option<Value> {
        self.consume_char('{');
        if depth > MAX_LUA_TABLE_DEPTH {
            self.dropped_over_nested = true;
            self.skip_current_table_body();
            tracing::warn!(
                depth,
                "dropped over-nested Lua table while parsing modinfo.lua"
            );
            return None;
        }
        let mut array = Vec::new();
        let mut object = Map::new();
        loop {
            self.skip_ws();
            if self.consume_char('}') {
                break;
            }
            if self.pos >= self.input.len() {
                return None;
            }

            let entry_start = self.pos;
            if self.consume_char('[') {
                let key = self.parse_bracket_key()?;
                self.skip_ws();
                if !self.consume_char('=') {
                    return None;
                }
                if let Some(value) = self.parse_value_at_depth(depth) {
                    object.insert(key, value);
                }
            } else if let Some(key) = self.parse_identifier() {
                self.skip_ws();
                if self.consume_char('=') {
                    if let Some(value) = self.parse_value_at_depth(depth) {
                        object.insert(key, value);
                    }
                } else {
                    self.pos = entry_start;
                    if let Some(value) = self.parse_value_at_depth(depth) {
                        array.push(value);
                    }
                }
            } else if let Some(value) = self.parse_value_at_depth(depth) {
                array.push(value);
            }

            self.skip_ws();
            self.consume_char(',');
            self.consume_char(';');
            if self.pos == entry_start {
                // Malformed tables from user-supplied modinfo.lua must never
                // spin forever. Drop the unrecognized byte and keep parsing the
                // rest of the file so one bad entry cannot block the handler.
                self.advance_char();
            }
        }

        if object.is_empty() {
            Some(Value::Array(array))
        } else if array.is_empty() {
            Some(Value::Object(object))
        } else {
            for (index, value) in array.into_iter().enumerate() {
                object.insert((index + 1).to_string(), value);
            }
            Some(Value::Object(object))
        }
    }

    fn parse_bracket_key(&mut self) -> Option<String> {
        self.skip_ws();
        let key = match self.peek_char()? {
            '"' | '\'' => self.parse_string()?,
            _ => {
                let start = self.pos;
                while self.peek_char().is_some_and(|character| character != ']') {
                    self.advance_char();
                }
                self.input[start..self.pos].trim().to_owned()
            }
        };
        self.skip_ws();
        if !self.consume_char(']') {
            return None;
        }
        Some(key)
    }

    fn skip_current_table_body(&mut self) {
        let mut depth = 1_usize;
        let mut quote = None::<char>;
        let mut escaped = false;
        while let Some(character) = self.advance_char() {
            if let Some(active_quote) = quote {
                if escaped {
                    escaped = false;
                    continue;
                }
                if character == '\\' {
                    escaped = true;
                    continue;
                }
                if character == active_quote {
                    quote = None;
                }
                continue;
            }
            match character {
                '"' | '\'' => quote = Some(character),
                '{' => depth = depth.saturating_add(1),
                '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    fn take_dropped_over_nested(&mut self) -> bool {
        let dropped = self.dropped_over_nested;
        self.dropped_over_nested = false;
        dropped
    }

    fn consume_char(&mut self, expected: char) -> bool {
        if self.peek_char() == Some(expected) {
            self.advance_char();
            true
        } else {
            false
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance_char(&mut self) -> Option<char> {
        let character = self.peek_char()?;
        self.pos += character.len_utf8();
        Some(character)
    }
}

fn select_translation_value(value: Value, lang: &str) -> Value {
    match value {
        Value::Object(mut object) => object
            .remove(lang)
            .or_else(|| object.remove("1"))
            .or_else(|| object.into_iter().next().map(|(_, value)| value))
            .unwrap_or(Value::Null),
        Value::Array(mut values) => {
            if values.is_empty() {
                Value::Null
            } else {
                values.remove(0)
            }
        }
        other => other,
    }
}
