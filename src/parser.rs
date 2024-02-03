use std::collections::HashMap;

const DEFAULT_ENTRY: &str = "entry";

use crate::{
    grammar::{self, Grammar},
    lexer::{Lexer, Token, TokenKinds},
};

pub struct Parser<'a> {
    pub text: &'a str,
    pub entry: String,
}

impl<'a> Parser<'a> {
    pub fn new(text: &'a str) -> Parser<'a> {
        Parser {
            text,
            entry: DEFAULT_ENTRY.to_string(),
        }
    }

    pub fn set_text(&mut self, text: &'a str) {
        self.text = text;
    }

    pub(crate) fn parse(
        &self,
        grammar: &Grammar,
        lexer: &Lexer,
    ) -> Result<ParseResult, ParseError> {
        println!(
            "{:?}",
            lexer
                .tokens
                .iter()
                .map(|t| t.kind.clone())
                .collect::<Vec<TokenKinds>>()
        );
        let mut cursor = Cursor { idx: 0 };
        let mut globals = Node::variables_from_grammar(&grammar.globals)?;
        let entry = match self.parse_node(grammar, lexer, &self.entry, &mut cursor, &mut globals) {
            Ok(node) => node,
            Err((err, _)) => return Err(err),
        };

        Ok(ParseResult {
            entry,
            globals,
            text: self.text,
        })
    }

    fn parse_node(
        &self,
        grammar: &Grammar,
        lexer: &Lexer,
        name: &str,
        cursor: &mut Cursor,
        globals: &mut HashMap<String, VariableKind>,
    ) -> Result<Node, (ParseError, Node)> {
        let mut node = match Node::from_grammar(grammar, name) {
            Ok(node) => node,
            Err(err) => return Err((err, Node::new(name.to_string()))),
        };
        node.first_token_idx = cursor.idx;
        // In case the node fails to parse, we want to restore the cursor to its original position
        let cursor_clone = cursor.clone();
        let rules = match grammar.nodes.get(name) {
            Some(node) => &node.rules,
            None => return Err((ParseError::NodeNotFound(name.to_string()), node)),
        };
        match self.parse_rules(
            grammar,
            lexer,
            rules,
            cursor,
            globals,
            &cursor_clone,
            &mut node,
        ) {
            Ok(_) => {}
            Err(err) => return Err((err, node)),
        }
        cursor.idx -= 1;
        node.last_token_idx = cursor.idx;

        Ok(node)
    }

    fn parse_rules(
        &self,
        grammar: &Grammar,
        lexer: &Lexer,
        rules: &Vec<grammar::Rule>,
        cursor: &mut Cursor,
        globals: &mut HashMap<String, VariableKind>,
        cursor_clone: &Cursor,
        node: &mut Node,
    ) -> Result<(), ParseError> {
        let mut i = 0;
        while i < rules.len() {
            let rule = &rules[i];
            match rule {
                grammar::Rule::Is {
                    token,
                    rules,
                    parameters,
                } => {
                    match self.match_token(grammar, lexer, token, cursor, globals, cursor_clone)? {
                        TokenCompare::Is(val) => {
                            self.parse_parameters(
                                grammar,
                                lexer,
                                parameters,
                                cursor,
                                globals,
                                cursor_clone,
                                node,
                                val,
                            )?;
                            cursor.idx += 1;
                            self.parse_rules(
                                grammar,
                                lexer,
                                rules,
                                cursor,
                                globals,
                                cursor_clone,
                                node,
                            )?;
                        }
                        TokenCompare::IsNot(err) => {
                            return Err(err);
                        }
                    };
                }
                grammar::Rule::Isnt {
                    token,
                    rules,
                    parameters: _,
                } => {
                    match self.match_token(grammar, lexer, token, cursor, globals, cursor_clone)? {
                        TokenCompare::Is(_) => {
                            err(
                                ParseError::ExpectedToNotBe(lexer.tokens[cursor.idx].kind.clone()),
                                cursor,
                                cursor_clone,
                            )?;
                        }
                        TokenCompare::IsNot(_) => {
                            self.parse_rules(
                                grammar,
                                lexer,
                                rules,
                                cursor,
                                globals,
                                cursor_clone,
                                node,
                            )?;
                        }
                    }
                }
                grammar::Rule::IsOneOf { tokens } => {
                    let mut found = false;
                    for (token, rules, parameters) in tokens {
                        use TokenCompare::*;
                        match self.match_token(
                            grammar,
                            lexer,
                            token,
                            cursor,
                            globals,
                            cursor_clone,
                        )? {
                            Is(val) => {
                                found = true;
                                self.parse_parameters(
                                    grammar,
                                    lexer,
                                    parameters,
                                    cursor,
                                    globals,
                                    cursor_clone,
                                    node,
                                    val,
                                )?;
                                cursor.idx += 1;
                                self.parse_rules(
                                    grammar,
                                    lexer,
                                    rules,
                                    cursor,
                                    globals,
                                    cursor_clone,
                                    node,
                                )?;
                                break;
                            }
                            IsNot(_) => {}
                        }
                    }
                    if !found {
                        err(
                            ParseError::ExpectedToken {
                                expected: TokenKinds::Text,
                                found: lexer.tokens[cursor.idx].kind.clone(),
                            },
                            cursor,
                            cursor_clone,
                        )?;
                    }
                }
                grammar::Rule::Maybe {
                    token,
                    is,
                    isnt,
                    parameters,
                } => {
                    println!("//maybe//");
                    use TokenCompare::*;
                    match self.match_token(grammar, lexer, token, cursor, globals, cursor_clone)? {
                        Is(val) => {
                            self.parse_parameters(
                                grammar,
                                lexer,
                                parameters,
                                cursor,
                                globals,
                                cursor_clone,
                                node,
                                val,
                            )?;
                            println!("--------");
                            cursor.idx += 1;
                            self.parse_rules(
                                grammar,
                                lexer,
                                is,
                                cursor,
                                globals,
                                cursor_clone,
                                node,
                            )?;
                        }
                        IsNot(_) => {
                            self.parse_rules(
                                grammar,
                                lexer,
                                isnt,
                                cursor,
                                globals,
                                cursor_clone,
                                node,
                            )?;
                        }
                    }
                }
                grammar::Rule::MaybeOneOf { is_one_of, isnt } => {
                    println!("//maybe one of//");
                    let mut found = false;
                    for (token, rules, parameters) in is_one_of {
                        use TokenCompare::*;
                        match self.match_token(
                            grammar,
                            lexer,
                            token,
                            cursor,
                            globals,
                            cursor_clone,
                        )? {
                            Is(val) => {
                                found = true;
                                self.parse_parameters(
                                    grammar,
                                    lexer,
                                    parameters,
                                    cursor,
                                    globals,
                                    cursor_clone,
                                    node,
                                    val,
                                )?;
                                cursor.idx += 1;
                                self.parse_rules(
                                    grammar,
                                    lexer,
                                    rules,
                                    cursor,
                                    globals,
                                    cursor_clone,
                                    node,
                                )?;
                                break;
                            }
                            IsNot(_) => {}
                        }
                    }
                    if !found {
                        self.parse_rules(
                            grammar,
                            lexer,
                            isnt,
                            cursor,
                            globals,
                            cursor_clone,
                            node,
                        )?;
                    }
                }
                grammar::Rule::While {
                    token,
                    rules,
                    parameters,
                } => {
                    println!("//while//");
                    while let TokenCompare::Is(val) =
                        self.match_token(grammar, lexer, token, cursor, globals, cursor_clone)?
                    {
                        self.parse_parameters(
                            grammar,
                            lexer,
                            parameters,
                            cursor,
                            globals,
                            cursor_clone,
                            node,
                            val,
                        )?;
                        cursor.idx += 1;
                        self.parse_rules(
                            grammar,
                            lexer,
                            rules,
                            cursor,
                            globals,
                            cursor_clone,
                            node,
                        )?;
                    }
                }
                grammar::Rule::Until {
                    token,
                    rules,
                    parameters,
                } => {
                    // search for the token and execute the rules when the token is found
                    println!("//until//");
                    while let TokenCompare::IsNot(_) =
                        self.match_token(grammar, lexer, token, cursor, globals, cursor_clone)?
                    {
                        cursor.idx += 1;
                        if cursor.idx >= lexer.tokens.len() {
                            return Err(ParseError::Eof);
                        }
                    }
                    self.parse_parameters(
                        grammar,
                        lexer,
                        parameters,
                        cursor,
                        globals,
                        cursor_clone,
                        node,
                        Nodes::Token(lexer.tokens[cursor.idx].clone()),
                    )?;
                    self.parse_rules(grammar, lexer, rules, cursor, globals, cursor_clone, node)?;
                }
                grammar::Rule::Command { command } => {
                    println!("//command//");
                    match command {
                        grammar::Commands::Compare {
                            left,
                            right,
                            comparison,
                            rules,
                        } => {
                            let left = match node.variables.get(left) {
                                Some(kind) => kind,
                                None => return Err(ParseError::VariableNotFound(left.to_string())),
                            };
                            let right = match node.variables.get(right) {
                                Some(kind) => kind,
                                None => {
                                    return Err(ParseError::VariableNotFound(right.to_string()))
                                }
                            };
                            let comparisons = match left {
                                VariableKind::Node(node_left) => {
                                    if let VariableKind::Node(node_right) = right {
                                        match (node_left, node_right) {
                                            (Some(Nodes::Node(left)), Some(Nodes::Node(right))) => {
                                                if left.name == right.name {
                                                    vec![grammar::Comparison::Equal]
                                                } else {
                                                    vec![grammar::Comparison::NotEqual]
                                                }
                                            }
                                            (
                                                Some(Nodes::Token(left)),
                                                Some(Nodes::Token(right)),
                                            ) => {
                                                if left == right {
                                                    vec![grammar::Comparison::Equal]
                                                } else {
                                                    vec![grammar::Comparison::NotEqual]
                                                }
                                            }
                                            (None, None) => {
                                                vec![grammar::Comparison::Equal]
                                            }
                                            _ => {
                                                vec![grammar::Comparison::NotEqual]
                                            }
                                        }
                                    } else {
                                        vec![grammar::Comparison::NotEqual]
                                    }
                                }
                                VariableKind::NodeList(_) => vec![grammar::Comparison::NotEqual],
                                VariableKind::Boolean(left) => {
                                    if let VariableKind::Boolean(right) = right {
                                        if *left == *right {
                                            vec![grammar::Comparison::Equal]
                                        } else {
                                            vec![grammar::Comparison::NotEqual]
                                        }
                                    } else {
                                        vec![grammar::Comparison::NotEqual]
                                    }
                                }
                                VariableKind::Number(left) => {
                                    if let VariableKind::Number(right) = right {
                                        let mut result = Vec::new();
                                        if *left == *right {
                                            result.push(grammar::Comparison::Equal);
                                            result.push(grammar::Comparison::GreaterThanOrEqual);
                                            result.push(grammar::Comparison::LessThanOrEqual);
                                        } else {
                                            result.push(grammar::Comparison::NotEqual);
                                            if *left > *right {
                                                result.push(grammar::Comparison::GreaterThan);
                                                result
                                                    .push(grammar::Comparison::GreaterThanOrEqual);
                                            }
                                            if *left < *right {
                                                result.push(grammar::Comparison::LessThan);
                                                result.push(grammar::Comparison::LessThanOrEqual);
                                            }
                                        }
                                        result
                                    } else {
                                        vec![grammar::Comparison::NotEqual]
                                    }
                                }
                            };
                            if comparisons.contains(comparison) {
                                self.parse_rules(
                                    grammar,
                                    lexer,
                                    rules,
                                    cursor,
                                    globals,
                                    cursor_clone,
                                    node,
                                )?;
                            }
                        }
                        grammar::Commands::Error { message } => {
                            Err(ParseError::Message(message.to_string()))?
                        }
                        grammar::Commands::HardError { set } => {
                            node.harderror = *set;
                        }
                    }
                }
            }
            i += 1;
        }
        Ok(())
    }

    fn match_token(
        &self,
        grammar: &Grammar,
        lexer: &Lexer,
        token: &grammar::MatchToken,
        cursor: &mut Cursor,
        globals: &mut HashMap<String, VariableKind>,
        cursor_clone: &Cursor,
    ) -> Result<TokenCompare, ParseError> {
        match token {
            grammar::MatchToken::Token(tok) => {
                let mut current_token = &lexer.tokens[cursor.idx];
                while current_token.kind == TokenKinds::Whitespace {
                    cursor.idx += 1;
                    current_token = &lexer.tokens[cursor.idx];
                }
                println!("{:?}", token);
                println!("{:?}", lexer.stringify(current_token));
                if *tok != current_token.kind {
                    return Ok(TokenCompare::IsNot(ParseError::ExpectedToken {
                        expected: tok.clone(),
                        found: current_token.kind.clone(),
                    }));
                }
                return Ok(TokenCompare::Is(Nodes::Token(current_token.clone())));
            }
            grammar::MatchToken::Node(node_name) => {
                println!("--{:?}--", node_name);
                match self.parse_node(grammar, lexer, node_name, cursor, globals) {
                    Ok(node) => return Ok(TokenCompare::Is(Nodes::Node(node))),
                    Err((err, node)) => match node.harderror {
                        true => return Err(err),
                        false => return Ok(TokenCompare::IsNot(err)),
                    },
                };
            }
            grammar::MatchToken::Word(word) => {
                let mut current_token = &lexer.tokens[cursor.idx];
                while current_token.kind == TokenKinds::Whitespace {
                    cursor.idx += 1;
                    current_token = &lexer.tokens[cursor.idx];
                }
                println!("{:?}", token);
                println!("{:?}", lexer.stringify(current_token));
                if let TokenKinds::Text = current_token.kind {
                    if word != &lexer.stringify(&current_token) {
                        return Ok(TokenCompare::IsNot(ParseError::ExpectedToken {
                            expected: TokenKinds::Text,
                            found: current_token.kind.clone(),
                        }));
                    }
                } else {
                    return Ok(TokenCompare::IsNot(ParseError::ExpectedWord {
                        expected: word.clone(),
                        found: current_token.kind.clone(),
                    }));
                }
                return Ok(TokenCompare::Is(Nodes::Token(current_token.clone())));
            }
            grammar::MatchToken::Enumerator(enumerator) => {
                println!("--{:?}--", enumerator);
                let enumerator = match grammar.enumerators.get(enumerator) {
                    Some(enumerator) => enumerator,
                    None => return Err(ParseError::EnumeratorNotFound(enumerator.to_string())),
                };
                let mut current_token = &lexer.tokens[cursor.idx];
                while current_token.kind == TokenKinds::Whitespace {
                    cursor.idx += 1;
                    current_token = &lexer.tokens[cursor.idx];
                }
                println!("{:?}", lexer.tokens[cursor.idx].kind);
                let mut i = 0;
                let token = loop {
                    if i >= enumerator.values.len() {
                        return Ok(TokenCompare::IsNot(ParseError::EnumeratorNotFound(
                            enumerator.name.clone(),
                        )));
                    }
                    let token = &enumerator.values[i];
                    if let TokenCompare::Is(val) =
                        self.match_token(grammar, lexer, token, cursor, globals, cursor_clone)?
                    {
                        break val;
                    }
                    i += 1;
                };
                return Ok(TokenCompare::Is(token));
            }
        }
    }

    fn parse_parameters(
        &self,
        _grammar: &Grammar,
        lexer: &Lexer,
        parameters: &Vec<grammar::Parameters>,
        cursor: &mut Cursor,
        globals: &mut HashMap<String, VariableKind>,
        _cursor_clone: &Cursor,
        node: &mut Node,
        value: Nodes,
    ) -> Result<(), ParseError> {
        for parameter in parameters {
            match parameter {
                grammar::Parameters::Set(name) => {
                    let kind = match node.variables.get_mut(name) {
                        Some(kind) => kind,
                        None => return Err(ParseError::VariableNotFound(name.to_string())),
                    };
                    match kind {
                        VariableKind::Node(single) => {
                            *single = Some(value.clone());
                        }
                        VariableKind::NodeList(list) => {
                            list.push(value.clone());
                        }
                        VariableKind::Boolean(_) => Err(ParseError::CannotSetVariable(
                            name.to_string(),
                            kind.clone(),
                        ))?,
                        VariableKind::Number(_) => Err(ParseError::CannotSetVariable(
                            name.to_string(),
                            kind.clone(),
                        ))?,
                    };
                }
                grammar::Parameters::Print(str) => println!("{}", str),
                grammar::Parameters::Debug(variable) => match variable {
                    Some(ident) => {
                        let kind = match node.variables.get(ident) {
                            Some(kind) => kind,
                            None => return Err(ParseError::VariableNotFound(ident.to_string())),
                        };
                        println!("{:?}", kind);
                    }
                    None => {
                        println!("{:?}", lexer.stringify(&lexer.tokens[cursor.idx]));
                    }
                },
                grammar::Parameters::Count(ident) => {
                    let kind = match node.variables.get_mut(ident) {
                        Some(kind) => kind,
                        None => return Err(ParseError::VariableNotFound(ident.to_string())),
                    };
                    match kind {
                        VariableKind::Node(_) => Err(ParseError::UncountableVariable(
                            ident.to_string(),
                            kind.clone(),
                        ))?,
                        VariableKind::NodeList(_) => Err(ParseError::UncountableVariable(
                            ident.to_string(),
                            kind.clone(),
                        ))?,
                        VariableKind::Boolean(_) => Err(ParseError::UncountableVariable(
                            ident.to_string(),
                            kind.clone(),
                        ))?,
                        VariableKind::Number(val) => {
                            *val += 1;
                        }
                    };
                }
                grammar::Parameters::True(variable) => {
                    let kind = match node.variables.get_mut(variable) {
                        Some(kind) => kind,
                        None => return Err(ParseError::VariableNotFound(variable.to_string())),
                    };
                    if let VariableKind::Boolean(val) = kind {
                        *val = true;
                    } else {
                        return Err(ParseError::UncountableVariable(
                            variable.to_string(),
                            kind.clone(),
                        ));
                    }
                }
                grammar::Parameters::False(variable) => {
                    let kind = match node.variables.get_mut(variable) {
                        Some(kind) => kind,
                        None => return Err(ParseError::VariableNotFound(variable.to_string())),
                    };
                    if let VariableKind::Boolean(val) = kind {
                        *val = false;
                    } else {
                        return Err(ParseError::UncountableVariable(
                            variable.to_string(),
                            kind.clone(),
                        ));
                    }
                }
                grammar::Parameters::Global(variable) => {
                    let kind = match globals.get_mut(variable) {
                        Some(kind) => kind,
                        None => return Err(ParseError::VariableNotFound(variable.to_string())),
                    };
                    match kind {
                        VariableKind::Node(single) => {
                            *single = Some(value.clone());
                        }
                        VariableKind::NodeList(list) => {
                            list.push(value.clone());
                        }
                        VariableKind::Boolean(_) => Err(ParseError::CannotSetVariable(
                            variable.to_string(),
                            kind.clone(),
                        ))?,
                        VariableKind::Number(_) => Err(ParseError::CannotSetVariable(
                            variable.to_string(),
                            kind.clone(),
                        ))?,
                    };
                }
                grammar::Parameters::CountGlobal(variable) => {
                    let kind = match globals.get_mut(variable) {
                        Some(kind) => kind,
                        None => return Err(ParseError::VariableNotFound(variable.to_string())),
                    };
                    match kind {
                        VariableKind::Node(_) => Err(ParseError::UncountableVariable(
                            variable.to_string(),
                            kind.clone(),
                        ))?,
                        VariableKind::NodeList(_) => Err(ParseError::UncountableVariable(
                            variable.to_string(),
                            kind.clone(),
                        ))?,
                        VariableKind::Boolean(_) => Err(ParseError::UncountableVariable(
                            variable.to_string(),
                            kind.clone(),
                        ))?,
                        VariableKind::Number(val) => {
                            *val += 1;
                        }
                    };
                }
                grammar::Parameters::TrueGlobal(variable) => {
                    let kind = match globals.get_mut(variable) {
                        Some(kind) => kind,
                        None => return Err(ParseError::VariableNotFound(variable.to_string())),
                    };
                    if let VariableKind::Boolean(val) = kind {
                        *val = true;
                    } else {
                        return Err(ParseError::UncountableVariable(
                            variable.to_string(),
                            kind.clone(),
                        ));
                    }
                }
                grammar::Parameters::FalseGlobal(variable) => {
                    let kind = match globals.get_mut(variable) {
                        Some(kind) => kind,
                        None => return Err(ParseError::VariableNotFound(variable.to_string())),
                    };
                    if let VariableKind::Boolean(val) = kind {
                        *val = false;
                    } else {
                        return Err(ParseError::UncountableVariable(
                            variable.to_string(),
                            kind.clone(),
                        ));
                    }
                }
                grammar::Parameters::HardError(value) => {
                    node.harderror = *value;
                }
                grammar::Parameters::Back(_) => todo!(),
                grammar::Parameters::Return => todo!(),
            }
        }
        Ok(())
    }
}

enum TokenCompare {
    Is(Nodes),
    IsNot(ParseError),
}

#[derive(Debug)]
pub struct ParseResult<'a> {
    pub entry: Node,
    pub globals: HashMap<String, VariableKind>,
    pub text: &'a str,
}

#[derive(Debug, Clone)]
pub enum Nodes {
    Node(Node),
    Token(Token),
}

#[derive(Debug, Clone)]
pub struct Node {
    name: String,
    variables: HashMap<String, VariableKind>,
    first_token_idx: usize,
    last_token_idx: usize,
    harderror: bool,
}

impl Node {
    pub fn new(name: String) -> Node {
        Node {
            name,
            variables: HashMap::new(),
            first_token_idx: 0,
            last_token_idx: 0,
            harderror: false,
        }
    }

    pub fn from_grammar(grammar: &Grammar, name: &str) -> Result<Node, ParseError> {
        let found = match grammar.nodes.get(name) {
            Some(node) => node,
            None => return Err(ParseError::NodeNotFound(name.to_string())),
        };
        let mut node = Node::new(found.name.clone());
        node.variables = Self::variables_from_grammar(&found.variables)?;
        Ok(node)
    }

    pub fn variables_from_grammar(
        variables: &HashMap<String, grammar::VariableKind>,
    ) -> Result<HashMap<String, VariableKind>, ParseError> {
        let mut result = HashMap::new();
        for (key, value) in variables {
            let var = match value {
                crate::grammar::VariableKind::Node => VariableKind::Node(None),
                crate::grammar::VariableKind::NodeList => VariableKind::NodeList(Vec::new()),
                crate::grammar::VariableKind::Boolean => VariableKind::Boolean(false),
                crate::grammar::VariableKind::Number => VariableKind::Number(0),
            };
            result.insert(key.clone(), var);
        }
        Ok(result)
    }
}

fn err(error: ParseError, cursor: &mut Cursor, cursor_clone: &Cursor) -> Result<(), ParseError> {
    *cursor = cursor_clone.clone();
    Err(error)
}

#[derive(Debug, Clone)]
pub enum VariableKind {
    Node(Option<Nodes>),
    NodeList(Vec<Nodes>),
    Boolean(bool),
    Number(i32),
}

pub enum ParseError {
    /// Parser not fully implemented - My fault
    ParserNotFullyImplemented,
    /// Node not found - Developer error
    NodeNotFound(String),
    /// Expected a token, found a token
    ExpectedToken {
        expected: TokenKinds,
        found: TokenKinds,
    },
    /// Expected a word, found a token
    ExpectedWord { expected: String, found: TokenKinds },
    /// Enumerator not found - Developer error
    EnumeratorNotFound(String),
    /// Expected to not be
    ExpectedToNotBe(TokenKinds),
    /// Variable not found - Developer error
    VariableNotFound(String),
    /// Uncountable variable - Developer error
    UncountableVariable(String, VariableKind),
    /// Cannot set variable - Developer error
    CannotSetVariable(String, VariableKind),
    /// Custom error message
    Message(String),
    /// Unexpected end of file
    Eof,
}

impl std::fmt::Debug for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ParseError::ParserNotFullyImplemented => write!(f, "Parser not fully implemented"),
            ParseError::NodeNotFound(name) => write!(f, "Node not found: {}", name),
            ParseError::ExpectedToken { expected, found } => {
                write!(f, "Expected token {:?}, found {:?}", expected, found)
            }
            ParseError::ExpectedWord { expected, found } => {
                write!(f, "Expected word {}, found {:?}", expected, found)
            }
            ParseError::EnumeratorNotFound(name) => write!(f, "Enumerator not found: {}", name),
            ParseError::ExpectedToNotBe(kind) => write!(f, "Expected to not be {:?}", kind),
            ParseError::VariableNotFound(name) => write!(f, "Variable not found: {}", name),
            ParseError::UncountableVariable(name, kind) => {
                write!(f, "Uncountable variable: {}<{:?}>", name, kind)
            }
            ParseError::CannotSetVariable(name, kind) => {
                write!(f, "Cannot set variable: {}<{:?}>", name, kind)
            }
            ParseError::Message(message) => write!(f, "{}", message),
            ParseError::Eof => write!(f, "Unexpected end of file"),
        }
    }
}

/// A cursor is used to keep track of the current position in the token stream and other useful information
#[derive(Clone)]
struct Cursor {
    /// Current index in the token stream
    idx: usize,
}
