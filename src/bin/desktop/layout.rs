use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Form {
    #[serde(skip_serializing)]
    Workspace,
    #[serde(alias = "Execute", alias = "UpdateRainfall")]
    NewRun,
    #[serde(alias = "Prepare")]
    RunAll,
    Jobs,
    Intake,
    Review,
    Forcing,
    RunBundle,
    Instructions,
    Geometry,
    Soil,
    Analysis,
    Bulletin,
    Automation,
}
impl Form {
    pub fn label(self, pt: bool) -> String {
        match self {
            Self::Workspace => super::text(
                pt,
                "Workspace Settings",
                "Configurações da área de trabalho",
            ),
            Self::NewRun => super::text(pt, "New run", "Nova execução"),
            Self::RunAll => super::text(pt, "Run experiment", "Executar experimento"),
            Self::Jobs => super::text(pt, "Jobs", "Tarefas"),
            Self::Intake => super::text(pt, "Import data", "Importar dados"),
            Self::Review => super::text(pt, "Observation review", "Revisão de observações"),
            Self::Forcing => super::text(pt, "Forcing and state", "Entradas e estado"),
            Self::RunBundle => super::text(pt, "Share runs", "Compartilhar execuções"),
            Self::Instructions => super::text(pt, "Review instructions", "Instruções de revisão"),
            Self::Geometry => super::text(pt, "Geometry editor", "Editor de geometria"),
            Self::Soil => super::text(pt, "Soil editor", "Editor de solo"),
            Self::Analysis => super::text(pt, "Run comparison", "Comparar execuções"),
            Self::Bulletin => super::text(pt, "Bulletins", "Boletins"),
            Self::Automation => super::text(pt, "Automation", "Automação"),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SideTab {
    Map,
    Charts,
    Data,
    #[serde(skip_serializing)]
    Rainfall,
    Files,
    Assistant,
    #[serde(skip_serializing)]
    Interface,
    Form(Form),
}

impl SideTab {
    pub fn label(self, pt: bool) -> String {
        match self {
            Self::Map => super::text(pt, "Map", "Mapa"),
            Self::Charts => super::text(pt, "Charts", "Gráficos"),
            Self::Data => super::text(pt, "Data", "Dados"),
            Self::Rainfall => super::text(pt, "Rainfall", "Chuva"),
            Self::Files => super::text(pt, "Files", "Arquivos"),
            Self::Assistant => super::text(pt, "Assistant", "Assistente"),
            Self::Interface => super::text(pt, "Interface Settings", "Configurações da interface"),
            Self::Form(form) => form.label(pt),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub tabs: Vec<SideTab>,
    pub active: SideTab,
    pub width: f32,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            tabs: vec![SideTab::Map, SideTab::Charts, SideTab::Data, SideTab::Files],
            active: SideTab::Map,
            width: 390.,
        }
    }
}

impl Layout {
    // Stored layouts from earlier desktop versions still contain these tab identifiers.
    pub fn restore_tabs(&mut self) {
        let mut seen = Vec::new();
        self.tabs.retain(|tab| {
            if matches!(
                tab,
                SideTab::Rainfall | SideTab::Interface | SideTab::Form(Form::Workspace)
            ) || seen.contains(tab)
            {
                return false;
            }
            seen.push(*tab);
            true
        });
        if !self.tabs.contains(&SideTab::Map) {
            self.tabs.insert(0, SideTab::Map);
        }
        if self.active == SideTab::Rainfall {
            self.active = SideTab::Map;
        }
        if !self.tabs.contains(&self.active) {
            self.activate_charts();
        }
    }

    pub fn open(&mut self, tab: SideTab) {
        if !self.tabs.contains(&tab) {
            self.tabs.push(tab);
        }
        self.active = tab;
    }

    pub fn form(&mut self, form: Form) {
        self.open(SideTab::Form(form));
    }

    pub fn close(&mut self, tab: SideTab) {
        if !matches!(tab, SideTab::Form(_) | SideTab::Assistant) {
            return;
        }
        self.tabs.retain(|item| *item != tab);
        if self.active == tab {
            self.activate_charts();
        }
    }

    fn activate_charts(&mut self) {
        if !self.tabs.contains(&SideTab::Charts) {
            let index = usize::from(self.tabs.first() == Some(&SideTab::Map));
            self.tabs.insert(index, SideTab::Charts);
        }
        self.active = SideTab::Charts;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_saved_tabs_removes_retired_entries_and_preserves_layout() {
        let mut layout: Layout = serde_json::from_str(
            r#"{
                "tabs": ["Data", "Rainfall", "Interface", {"Form":"Workspace"},
                         {"Form":"Execute"}, "Files", "Data"],
                "active": {"Form":"Workspace"},
                "width": 428
            }"#,
        )
        .unwrap();
        layout.restore_tabs();

        assert_eq!(
            layout.tabs,
            [
                SideTab::Map,
                SideTab::Charts,
                SideTab::Data,
                SideTab::Form(Form::NewRun),
                SideTab::Files,
            ]
        );
        assert_eq!(layout.active, SideTab::Charts);
        assert_eq!(layout.width, 428.);
        layout.open(SideTab::Form(Form::NewRun));
        layout.restore_tabs();
        assert_eq!(layout.active, SideTab::Form(Form::NewRun));
        layout.active = SideTab::Rainfall;
        layout.restore_tabs();
        layout.close(SideTab::Map);
        assert_eq!(layout.active, SideTab::Map);
        assert_eq!(
            layout
                .tabs
                .iter()
                .filter(|tab| **tab == SideTab::Map)
                .count(),
            1
        );
    }
}
