use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingKind {
    Model,
    Effort,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingChoice {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentSetting {
    pub selected: String,
    pub choices: Vec<SettingChoice>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentSettings {
    pub model: Option<AgentSetting>,
    pub effort: Option<AgentSetting>,
}

impl AgentSettings {
    pub fn get(&self, kind: SettingKind) -> Option<&AgentSetting> {
        match kind {
            SettingKind::Model => self.model.as_ref(),
            SettingKind::Effort => self.effort.as_ref(),
        }
    }
}

struct Select {
    id: String,
    setting: AgentSetting,
}

#[derive(Default)]
pub(super) struct Configuration {
    model: Option<Select>,
    effort: Option<Select>,
    legacy_model: Option<AgentSetting>,
}

impl Configuration {
    pub(super) fn load(result: &Value) -> Self {
        let mut configuration = Self {
            legacy_model: legacy_model(&result["models"]),
            ..Self::default()
        };
        if let Some(options) = result["configOptions"].as_array() {
            configuration.update(options);
        }
        configuration
    }

    pub(super) fn update(&mut self, options: &[Value]) {
        self.model = select(options, "model", &["model"]);
        self.effort = select(
            options,
            "thought_level",
            &["thought_level", "reasoning_effort", "effort"],
        );
    }

    pub(super) fn snapshot(&self) -> AgentSettings {
        AgentSettings {
            model: self
                .model
                .as_ref()
                .map(|option| &option.setting)
                .or(self.legacy_model.as_ref())
                .cloned(),
            effort: self.effort.as_ref().map(|option| option.setting.clone()),
        }
    }

    // None denotes the older model selection method, available only with an advertised model list.
    pub(super) fn target(&self, kind: SettingKind, value: &str) -> Result<Option<&str>, String> {
        let (option, legacy) = match kind {
            SettingKind::Model => (self.model.as_ref(), self.legacy_model.as_ref()),
            SettingKind::Effort => (self.effort.as_ref(), None),
        };
        let setting = option
            .map(|option| &option.setting)
            .or(legacy)
            .ok_or("The agent does not offer this setting")?;
        if !setting.choices.iter().any(|choice| choice.id == value) {
            return Err("The agent no longer offers that value".into());
        }
        Ok(option.map(|option| option.id.as_str()))
    }

    pub(super) fn confirm_legacy_model(&mut self, value: String) {
        if let Some(model) = &mut self.legacy_model {
            model.selected = value;
        }
    }
}

fn select(options: &[Value], category: &str, ids: &[&str]) -> Option<Select> {
    let parse = |option: &Value| {
        if option["type"].as_str() != Some("select") {
            return None;
        }
        let mut choices = Vec::new();
        for entry in option["options"].as_array()? {
            if let Some(group) = entry["options"].as_array() {
                choices.extend(
                    group
                        .iter()
                        .filter_map(|choice| parse_choice(choice, "value")),
                );
            } else if let Some(choice) = parse_choice(entry, "value") {
                choices.push(choice);
            }
        }
        Some(Select {
            id: nonempty(&option["id"])?.into(),
            setting: AgentSetting {
                selected: nonempty(&option["currentValue"])?.into(),
                choices,
            },
        })
    };
    options
        .iter()
        .filter(|option| option["category"].as_str() == Some(category))
        .find_map(parse)
        .or_else(|| {
            options
                .iter()
                .filter(|option| {
                    option["category"].is_null()
                        && option["id"].as_str().is_some_and(|id| ids.contains(&id))
                })
                .find_map(parse)
        })
}

fn legacy_model(models: &Value) -> Option<AgentSetting> {
    Some(AgentSetting {
        selected: nonempty(&models["currentModelId"])?.into(),
        choices: models["availableModels"]
            .as_array()?
            .iter()
            .filter_map(|choice| parse_choice(choice, "modelId"))
            .collect(),
    })
}

fn parse_choice(choice: &Value, id: &str) -> Option<SettingChoice> {
    let id = nonempty(&choice[id])?;
    Some(SettingChoice {
        id: id.into(),
        label: nonempty(&choice["name"]).unwrap_or(id).into(),
    })
}

fn nonempty(value: &Value) -> Option<&str> {
    value.as_str().filter(|value| !value.is_empty())
}
