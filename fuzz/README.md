# Optional fuzz harnesses

Harnesses cover retained parser/query/selector/Markdown/record and CLI surfaces.
Retired bundle targets and corpus are removed. Compile with `cargo check --locked
-p htmlcut-fuzz --all-targets`. No live fuzzing or mutation campaign is part of this
change or its required checks. Authored adversarial regression/resource tests remain
mandatory. Fuzz execution was explicitly excluded by the user for this work.
