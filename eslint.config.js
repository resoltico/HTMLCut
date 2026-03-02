export default [
    {
        ignores: ["eslint.config.js"],
    },
    {
        languageOptions: {
            ecmaVersion: "latest",
            sourceType: "module",
            globals: {
                console: "readonly",
                process: "readonly",
                performance: "readonly",
                fetch: "readonly",
                TextDecoder: "readonly",
                AbortSignal: "readonly",
                URL: "readonly",
            }
        },
        rules: {
            "indent": ["error", 4],
            "no-undef": "error",
            "no-unused-vars": ["error", { "varsIgnorePattern": "^_" }],
            "eqeqeq": ["error", "always"],
            "no-var": "error",
            "prefer-const": "error",
            "curly": ["error", "all"],
            "require-await": "error",
            "no-shadow": "error",
            "no-throw-literal": "error",
            "prefer-template": "error",
            "semi": ["error", "always"],
            "no-console": "off",
            "no-magic-numbers": [
                "error",
                {
                    ignore: [-1, 0, 1, 2, 16, 50, 1024],
                    ignoreArrayIndexes: true,
                    enforceConst: true,
                    detectObjects: true
                }
            ]
        }
    },
    {
        // Test files: relax no-magic-numbers — HTTP status codes, byte sizes, and
        // assertion values are contextually obvious and named where it adds clarity.
        // This override must be last so it takes precedence over the global rules above.
        files: ["test/**/*.js"],
        rules: {
            "no-magic-numbers": "off",
        },
    },
];
