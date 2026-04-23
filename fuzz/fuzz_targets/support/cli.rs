pub fn drive(input: &[u8]) {
    let (structured_request, equals_syntax, positional) = decode_input(input);
    let mut args = vec!["htmlcut".to_owned(), "select".to_owned(), positional];

    if structured_request {
        if equals_syntax {
            args.push("--value=structured".to_owned());
        } else {
            args.push("--value".to_owned());
            args.push("structured".to_owned());
        }
    }

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = htmlcut_cli::run(args, &mut stdout, &mut stderr);
    let stdout = String::from_utf8_lossy(&stdout);
    let stderr = String::from_utf8_lossy(&stderr);

    assert_eq!(exit_code, htmlcut_cli::EXIT_CODE_USAGE);
    if structured_request {
        assert!(stderr.is_empty());
        assert!(stdout.contains("\"tool\": \"htmlcut\""));
        assert!(stdout.contains("\"code\": \"CLI_PARSE_ERROR\""));
    } else {
        assert!(stdout.is_empty());
        assert!(!stderr.is_empty());
        assert!(!stderr.contains("\"tool\":"));
    }
}

fn decode_input(input: &[u8]) -> (bool, bool, String) {
    let mode = input.first().copied().unwrap_or(b'0');
    let raw_positional = String::from_utf8_lossy(input.get(1..).unwrap_or_default());
    let mut positional = raw_positional.replace('\0', "");
    if positional.is_empty() {
        positional = "inspect".to_owned();
    }
    if positional.starts_with('-') {
        positional.insert(0, 'x');
    }

    let structured_request = matches!(mode, b'1' | b'2' | b'3');
    let equals_syntax = matches!(mode, b'2' | b'3');

    (structured_request, equals_syntax, positional)
}
