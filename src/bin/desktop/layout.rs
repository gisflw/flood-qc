use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Form {
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
            Self::Workspace => super::text(pt, "Workspace Settings", "Configurações"),
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
    Charts,
    Data,
    Rainfall,
    Files,
    Interface,
    Form(Form),
}

impl SideTab {
    pub fn label(self, pt: bool) -> String {
        match self {
            Self::Charts => super::text(pt, "Charts", "Gráficos"),
            Self::Data => super::text(pt, "Data", "Dados"),
            Self::Rainfall => super::text(pt, "Rainfall", "Chuva"),
            Self::Files => super::text(pt, "Files", "Arquivos"),
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
            tabs: vec![
                SideTab::Charts,
                SideTab::Data,
                SideTab::Rainfall,
                SideTab::Files,
            ],
            active: SideTab::Charts,
            width: 390.,
        }
    }
}

impl Layout {
    pub fn deduplicate_tabs(&mut self) {
        let mut seen = Vec::new();
        self.tabs.retain(|tab| {
            if seen.contains(tab) {
                return false;
            }
            seen.push(*tab);
            true
        });
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
        self.tabs.retain(|item| *item != tab);
        if self.active == tab {
            self.active = SideTab::Charts;
        }
    }
}
