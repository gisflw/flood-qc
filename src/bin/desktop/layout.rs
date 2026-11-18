use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Form {
    Workspace,
    Prepare,
    Execute,
    RunAll,
    Jobs,
}
impl Form {
    pub fn label(self, pt: bool) -> String {
        match self {
            Self::Workspace => super::text(pt, "Workspace Settings", "Configurações"),
            Self::Prepare => super::text(pt, "Prepare inputs", "Preparar entradas"),
            Self::Execute => super::text(pt, "Execute MGB", "Executar MGB"),
            Self::RunAll => super::text(pt, "Run All", "Executar tudo"),
            Self::Jobs => super::text(pt, "Jobs", "Tarefas"),
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
