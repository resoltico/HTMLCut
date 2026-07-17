use super::*;

impl Plan {
    /// Builds one extraction plan with the v1 schema identity.
    pub fn new(
        strategy: PlanStrategy,
        selection: Selection,
        output: Output,
        rendering: Rendering,
    ) -> Self {
        Self {
            schema_name: PLAN_SCHEMA_NAME.to_owned(),
            schema_version: PLAN_SCHEMA_VERSION,
            interop_profile: INTEROP_V1_PROFILE.to_owned(),
            strategy,
            selection,
            output,
            rendering,
            dom_canonicalization: None,
            extensions: None,
        }
    }

    /// Sets the detached-clone canonicalization policy for this CSS-selector plan.
    pub fn with_dom_canonicalization(mut self, dom_canonicalization: DomCanonicalization) -> Self {
        self.dom_canonicalization = Some(dom_canonicalization);
        self
    }

    /// Validates the schema identity and semantic invariants for this plan.
    pub fn validate(&self) -> Result<(), ContractError> {
        validate_schema_identity(
            &self.schema_name,
            PLAN_SCHEMA_NAME,
            self.schema_version,
            PLAN_SCHEMA_VERSION,
            &self.interop_profile,
            INTEROP_V1_PROFILE,
        )?;
        self.strategy.validate()?;
        self.output.validate_for_strategy(self.strategy.kind())?;
        self.validate_dom_canonicalization()
    }

    fn validate_dom_canonicalization(&self) -> Result<(), ContractError> {
        let Some(dom_canonicalization) = &self.dom_canonicalization else {
            return Ok(());
        };

        if self.strategy.kind() != StrategyKind::CssSelector {
            return Err(ContractError::DomCanonicalizationRequiresCssSelector);
        }

        if let Output::Attribute { name } = &self.output
            && dom_canonicalization.ignores_attribute(name.as_str())
        {
            return Err(ContractError::DomCanonicalizationIgnoresMeasuredAttribute {
                attribute: name.as_str().to_owned(),
            });
        }

        if !matches!(self.output, Output::Text | Output::Structured) {
            return Err(
                ContractError::DomCanonicalizationRequiresComparisonTextOutput {
                    output_kind: self.output.kind(),
                },
            );
        }

        Ok(())
    }

    /// Serializes this plan with the stable JSON profile.
    pub fn stable_json(&self) -> Result<String, ContractError> {
        self.validate()?;
        super::super::super::stable_json::stable_json_v1(self)
    }

    /// Computes the SHA-256 digest of this exact plan document.
    pub fn digest_sha256(&self) -> Result<String, ContractError> {
        self.validate()?;
        digest_stable_json(self)
    }
}
