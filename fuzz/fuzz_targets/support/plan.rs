// SPDX-License-Identifier: MPL-2.0
use htmlcut_core::{CompiledPlan, ExtractionPlan, canonical_json};

pub fn drive(data: &[u8]) {
    if data.len() > 32768 {
        return;
    }
    if let Ok(plan) = ExtractionPlan::from_json(data) {
        let serialized = canonical_json(&plan).unwrap();
        assert_eq!(
            plan,
            ExtractionPlan::from_json(serialized.as_bytes()).unwrap()
        );
        if let Ok(compiled) = CompiledPlan::compile(&plan)
            && let Some(document) = crate::snapshot::document(
                "<main><p id='amount' data-x=''>180</p><template>text</template></main>",
            )
        {
            let _ = document.execute(&compiled);
        }
    }
}
