use std::collections::HashMap;

use rparse::parser::VariableKind;
use rparse::{api::*, grammar};
use rparse::{grammar::*, lexer::*, Parser};

pub fn gen_parser() -> Parser {
    let mut parser = Parser::new();

    let tokens = vec![
        "+=".to_string(),
        "-=".to_string(),
        "*=".to_string(),
        "/=".to_string(),
        "+".to_string(),
        "-".to_string(),
        "*".to_string(),
        "/".to_string(),
        "(".to_string(),
        ")".to_string(),
        "{".to_string(),
        "}".to_string(),
        "[".to_string(),
        "]".to_string(),
        "<=".to_string(),
        ">=".to_string(),
        "<".to_string(),
        ">".to_string(),
        "==".to_string(),
        "=".to_string(),
        "!=".to_string(),
        "!".to_string(),
        "&&".to_string(),
        "||".to_string(),
        "?".to_string(),
        ":".to_string(),
        ".".to_string(),
        ";".to_string(),
        ",".to_string(),
        "\"".to_string(),
        "'".to_string(),
        "#".to_string(),
    ];
    parser.lexer.add_tokens(&tokens);

    let preprocessor: Preprocessor = |text, tokens| {
        let mut new_tokens = vec![];
        let mut i = 0;
        'main: while i < tokens.len() {
            let token = &tokens[i];
            match &token.kind {
                TokenKinds::Text => {
                    let text = &text[token.index..token.index + token.len];

                    // test for number
                    // strip suffix character (u, i, f, etc.)
                    let c = text.chars().last().unwrap();
                    let text1 = if c.is_alphabetic() {
                        &text[..text.len() - 1]
                    } else {
                        text
                    };
                    match text1.parse::<u64>() {
                        Ok(_) => {
                            if tokens[i + 1].kind != TokenKinds::Token(".".to_string()) {
                                // it's an integer (but could be another type if it has a suffix)
                                new_tokens.push(Token {
                                    kind: TokenKinds::Complex(
                                        match c {
                                            'u' => "uint",
                                            'i' => "int",
                                            'f' => "float",
                                            'c' => "char",
                                            _ => "int",
                                        }
                                        .to_string(),
                                    ),
                                    index: token.index,
                                    len: token.len,
                                    location: token.location.clone(),
                                });
                                i += 1;
                                continue 'main;
                            }
                            // it's a float (suffix is not allowed)
                            match tokens[i + 2].kind {
                                TokenKinds::Text => {
                                    let token = &tokens[i + 2];
                                    let text = &text[token.index..token.index + token.len];
                                    match text.parse::<f64>() {
                                        Ok(_) => {
                                            // it's a float with a decimal value
                                            new_tokens.push(Token {
                                                index: token.index,
                                                len: tokens[i].len
                                                    + tokens[i + 1].len
                                                    + tokens[i + 2].len,
                                                location: token.location.clone(),
                                                kind: TokenKinds::Complex("float".to_string()),
                                            });
                                            i += 3;
                                            continue 'main;
                                        }
                                        Err(_) => {
                                            // it's a float without a decimal value even though it has a decimal point (error)
                                            Err(PreprocessorError {
                                                message: "Expected a float".to_string(),
                                                location: token.location.clone(),
                                                len: token.len
                                                    + tokens[i + 1].len
                                                    + tokens[i + 2].len,
                                            })?
                                        }
                                    }
                                }
                                _ => {
                                    // it's a float without a decimal value
                                    new_tokens.push(Token {
                                        index: token.index,
                                        len: token.len + tokens[i + 1].len,
                                        location: token.location.clone(),
                                        kind: TokenKinds::Complex("float".to_string()),
                                    });
                                    i += 2;
                                    continue 'main;
                                }
                            }
                        }
                        Err(_) => (),
                    }
                    new_tokens.push(token.clone());
                }
                TokenKinds::Token(tok) => match tok.as_str() {
                    "\"" => {
                        let mut j = i + 1;
                        while j < tokens.len() {
                            let current = &tokens[j];
                            if current.kind == TokenKinds::Token("\"".to_string())
                                && tokens[j - 1].kind != TokenKinds::Token("\\".to_string())
                            {
                                new_tokens.push(Token {
                                    index: token.index,
                                    len: current.index - token.index + current.len,
                                    location: token.location.clone(),
                                    kind: TokenKinds::Complex("string".to_string()),
                                });
                                i = j + 1;
                                continue 'main;
                            }
                            j += 1;
                        }
                        let current = &tokens[j - 1];
                        Err(PreprocessorError {
                            message: "Expected a closing quote".to_string(),
                            location: token.location.clone(),
                            len: current.index - token.index + current.len,
                        })?;
                    }
                    _ => {
                        new_tokens.push(token.clone());
                    }
                },
                TokenKinds::Whitespace => (),
                TokenKinds::Control(ControlTokenKind::Eol) => (),
                _ => {
                    new_tokens.push(token.clone());
                }
            }
            i += 1;
        }
        Ok(new_tokens)
    };
    parser.lexer.preprocessors.push(preprocessor);

    let operators = Enumerator {
        name: "operators".to_string(),
        values: vec![
            MatchToken::Token(TokenKinds::Token("+=".to_string())),
            MatchToken::Token(TokenKinds::Token("-=".to_string())),
            MatchToken::Token(TokenKinds::Token("*=".to_string())),
            MatchToken::Token(TokenKinds::Token("/=".to_string())),
            MatchToken::Token(TokenKinds::Token("+".to_string())),
            MatchToken::Token(TokenKinds::Token("-".to_string())),
            MatchToken::Token(TokenKinds::Token("*".to_string())),
            MatchToken::Token(TokenKinds::Token("/".to_string())),
            MatchToken::Token(TokenKinds::Token("<=".to_string())),
            MatchToken::Token(TokenKinds::Token(">=".to_string())),
            MatchToken::Token(TokenKinds::Token("<".to_string())),
            MatchToken::Token(TokenKinds::Token(">".to_string())),
            MatchToken::Token(TokenKinds::Token("==".to_string())),
            MatchToken::Token(TokenKinds::Token("=".to_string())),
            MatchToken::Token(TokenKinds::Token("!=".to_string())),
            MatchToken::Token(TokenKinds::Token("&&".to_string())),
        ],
    };
    parser
        .grammar
        .enumerators
        .insert(operators.name.clone(), operators);

    let keywords = Enumerator {
        name: "keywords".to_string(),
        values: vec![
            MatchToken::Word("if".to_string()),
            MatchToken::Word("else".to_string()),
            MatchToken::Word("while".to_string()),
            MatchToken::Word("for".to_string()),
            MatchToken::Word("return".to_string()),
            MatchToken::Word("break".to_string()),
            MatchToken::Word("continue".to_string()),
            MatchToken::Word("fun".to_string()),
            MatchToken::Word("let".to_string()),
            MatchToken::Word("const".to_string()),
            MatchToken::Word("enum".to_string()),
            MatchToken::Word("struct".to_string()),
            MatchToken::Word("impl".to_string()),
            MatchToken::Word("trait".to_string()),
            MatchToken::Word("type".to_string()),
            MatchToken::Word("use".to_string()),
            MatchToken::Word("as".to_string()),
            MatchToken::Word("error".to_string()),
            MatchToken::Word("yeet".to_string()),
            MatchToken::Word("delete".to_string()),
            MatchToken::Word("switch".to_string()),
            MatchToken::Word("new".to_string()),
            MatchToken::Word("try".to_string()),
            MatchToken::Word("catch".to_string()),
        ],
    };
    parser
        .grammar
        .enumerators
        .insert(keywords.name.clone(), keywords);

    let unary_operators = Enumerator {
        name: "unary_operators".to_string(),
        values: vec![
            MatchToken::Token(TokenKinds::Token("!".to_string())),
            MatchToken::Token(TokenKinds::Token("-".to_string())),
        ],
    };
    parser
        .grammar
        .enumerators
        .insert(unary_operators.name.clone(), unary_operators);

    let setting_operators = Enumerator {
        name: "setting_operators".to_string(),
        values: vec![
            MatchToken::Token(TokenKinds::Token("=".to_string())),
            MatchToken::Token(TokenKinds::Token("+=".to_string())),
            MatchToken::Token(TokenKinds::Token("-=".to_string())),
            MatchToken::Token(TokenKinds::Token("*=".to_string())),
            MatchToken::Token(TokenKinds::Token("/=".to_string())),
        ],
    };
    parser
        .grammar
        .enumerators
        .insert(setting_operators.name.clone(), setting_operators);

    let types = Enumerator {
        name: "types".to_string(),
        values: vec![
            MatchToken::Word("char".to_string()),
            MatchToken::Word("int".to_string()),
            MatchToken::Word("float".to_string()),
            MatchToken::Word("bool".to_string()),
            MatchToken::Word("string".to_string()),
            MatchToken::Word("uint".to_string()),
        ],
    };
    parser.grammar.enumerators.insert(types.name.clone(), types);

    let numbers = Enumerator {
        name: "numbers".to_string(),
        values: vec![
            MatchToken::Token(TokenKinds::Complex("int".to_string())),
            MatchToken::Token(TokenKinds::Complex("float".to_string())),
            MatchToken::Token(TokenKinds::Complex("uint".to_string())),
        ],
    };
    parser
        .grammar
        .enumerators
        .insert(numbers.name.clone(), numbers);

    let literals = Enumerator {
        name: "literals".to_string(),
        values: vec![
            MatchToken::Token(TokenKinds::Complex("string".to_string())),
            MatchToken::Token(TokenKinds::Complex("char".to_string())),
            MatchToken::Enumerator("numbers".to_string()),
        ],
    };
    parser
        .grammar
        .enumerators
        .insert(literals.name.clone(), literals);

    let mut variables = HashMap::new();
    variables.insert("list".to_string(), grammar::VariableKind::NodeList);
    let entry = Node {
        name: "entry".to_string(),
        rules: vec![
            Rule::Loop {
                rules: vec![Rule::IsOneOf {
                    tokens: vec![
                        OneOf {
                            token: MatchToken::Node("KWImport".to_string()),
                            rules: vec![],
                            parameters: vec![Parameters::Set("list".to_string())],
                        },
                        OneOf {
                            token: MatchToken::Node("KWFunction".to_string()),
                            rules: vec![],
                            parameters: vec![Parameters::Set("list".to_string())],
                        },
                        OneOf {
                            token: MatchToken::Token(TokenKinds::Control(ControlTokenKind::Eof)),
                            rules: vec![],
                            parameters: vec![Parameters::Goto("end".to_string()), Parameters::Debug(None)],
                        },
                    ],
                }],
            },
            Rule::Command {
                command: Commands::Label {
                    name: "end".to_string(),
                },
            },
        ],
        variables,
    };
    parser.grammar.nodes.insert(entry.name.clone(), entry);

    let mut variables = HashMap::new();
    variables.insert("file".to_string(), grammar::VariableKind::Node);
    variables.insert("alias".to_string(), grammar::VariableKind::Node);
    let import = Node {
        name: "KWImport".to_string(),
        rules: vec![
            Rule::Is {
                token: MatchToken::Word("import".to_string()),
                rules: vec![Rule::Is {
                    token: MatchToken::Token(TokenKinds::Complex("string".to_string())),
                    rules: vec![],
                    parameters: vec![Parameters::Set("file".to_string())],
                }],
                parameters: vec![Parameters::HardError(true)],
            },
            Rule::Maybe {
                token: MatchToken::Word("as".to_string()),
                is: vec![Rule::Is {
                    token: MatchToken::Token(TokenKinds::Text),
                    rules: vec![],
                    parameters: vec![Parameters::Set("alias".to_string())],
                }],
                isnt: vec![],
                parameters: vec![],
            },
        ],
        variables,
    };
    parser.grammar.nodes.insert(import.name.clone(), import);

    let mut variables = HashMap::new();
    variables.insert("identifier".to_string(), grammar::VariableKind::Node);
    variables.insert("parameters".to_string(), grammar::VariableKind::NodeList);
    variables.insert("return_type".to_string(), grammar::VariableKind::Node);
    variables.insert("body".to_string(), grammar::VariableKind::Node);
    let function = Node {
        name: "KWFunction".to_string(),
        rules: vec![
            Rule::Command { command: Commands::Print { message: "ahoj".to_string() } },
            Rule::Is {
                token: MatchToken::Word("fun".to_string()),
                rules: vec![],
                parameters: vec![Parameters::HardError(true), Parameters::Debug(None)],
            },
            Rule::Command { command: Commands::Print { message: "ahoj".to_string() } },
            Rule::Is {
                token: MatchToken::Token(TokenKinds::Text),
                rules: vec![],
                parameters: vec![Parameters::Set("identifier".to_string())],
            },
            Rule::Is {
                token: MatchToken::Token(TokenKinds::Token("(".to_string())),
                rules: vec![],
                parameters: vec![],
            },
            Rule::Maybe {
                token: MatchToken::Node("parameter".to_string()),
                is: vec![Rule::While {
                    token: MatchToken::Token(TokenKinds::Token(",".to_string())),
                    rules: vec![Rule::Is {
                        token: MatchToken::Node("parameter".to_string()),
                        rules: vec![],
                        parameters: vec![Parameters::Set("parameters".to_string())],
                    }],
                    parameters: vec![],
                }],
                isnt: vec![],
                parameters: vec![Parameters::Set("parameters".to_string())],
            },
            Rule::Is {
                token: MatchToken::Token(TokenKinds::Token(")".to_string())),
                rules: vec![],
                parameters: vec![],
            },
            Rule::Maybe {
                token: MatchToken::Token(TokenKinds::Token(":".to_string())),
                is: vec![Rule::Is {
                    token: MatchToken::Node("type".to_string()),
                    rules: vec![],
                    parameters: vec![Parameters::Set("return_type".to_string())],
                }],
                isnt: vec![],
                parameters: vec![],
            },
            Rule::Is {
                token: MatchToken::Node("block".to_string()),
                rules: vec![],
                parameters: vec![Parameters::Set("body".to_string())],
            },
        ],
        variables,
    };
    parser.grammar.nodes.insert(function.name.clone(), function);

    let mut variables = HashMap::new();
    variables.insert("nodes".to_string(), grammar::VariableKind::NodeList);
    let block = Node {
        name: "block".to_string(),
        rules: vec![
            Rule::Is {
                token: MatchToken::Token(TokenKinds::Token("{".to_string())),
                rules: vec![],
                parameters: vec![Parameters::HardError(true)],
            },
            // will implement later
            Rule::Until {
                token: MatchToken::Token(TokenKinds::Token("}".to_string())),
                rules: vec![],
                parameters: vec![],
            },
        ],
        variables,
    };
    parser.grammar.nodes.insert(block.name.clone(), block);

    let mut variables = HashMap::new();
    variables.insert("identifier".to_string(), grammar::VariableKind::Node);
    variables.insert("type".to_string(), grammar::VariableKind::Node);
    let type_specifier = Node {
        name: "parameter".to_string(),
        rules: vec![
            Rule::Is {
                token: MatchToken::Token(TokenKinds::Text),
                rules: vec![],
                parameters: vec![Parameters::Set("identifier".to_string())],
            },
            Rule::Is {
                token: MatchToken::Token(TokenKinds::Token(":".to_string())),
                rules: vec![],
                parameters: vec![],
            },
            Rule::Is {
                token: MatchToken::Node("type".to_string()),
                rules: vec![],
                parameters: vec![Parameters::Set("type".to_string())],
            },
        ],
        variables,
    };
    parser
        .grammar
        .nodes
        .insert(type_specifier.name.clone(), type_specifier);

    let mut variables = HashMap::new();
    variables.insert("refs".to_string(), grammar::VariableKind::Number);
    variables.insert("path".to_string(), grammar::VariableKind::Node);
    let type_ = Node {
        name: "type".to_string(),
        rules: vec![
            Rule::While {
                token: MatchToken::Token(TokenKinds::Token("&".to_string())),
                rules: vec![],
                parameters: vec![Parameters::Increment("refs".to_string())],
            },
            Rule::Is {
                token: MatchToken::Node("path".to_string()),
                rules: vec![],
                parameters: vec![Parameters::Set("path".to_string())],
            },
        ],
        variables,
    };
    parser.grammar.nodes.insert(type_.name.clone(), type_);

    let mut variables = HashMap::new();
    variables.insert("nodes".to_string(), grammar::VariableKind::NodeList);
    let path = Node {
        name: "path".to_string(),
        rules: vec![
            Rule::Is {
                token: MatchToken::Token(TokenKinds::Text),
                rules: vec![],
                parameters: vec![Parameters::Set("nodes".to_string())],
            },
            Rule::While {
                token: MatchToken::Token(TokenKinds::Token(".".to_string())),
                rules: vec![Rule::Is {
                    token: MatchToken::Token(TokenKinds::Text),
                    rules: vec![],
                    parameters: vec![Parameters::Set("nodes".to_string())],
                }],
                parameters: vec![],
            },
        ],
        variables,
    };
    parser.grammar.nodes.insert(path.name.clone(), path);

    parser
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let parser = gen_parser();

        let test_string = 
r##"import "#io" as io

fun main() {
    io.println("Hello, World!")
}
"##;

        let tokens = parser.lexer.lex_utf8(test_string);
        let ast = parser.parse(&tokens.unwrap(), test_string);

        panic!("{:#?}", ast.unwrap());
    }
}