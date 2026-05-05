use dioxus::prelude::*;

const APP_STYLES: &str = r#"
:root {
  color-scheme: light;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
  background: #f5f5f7;
  color: #1c1c1e;
}

.app-shell {
  margin: 0 auto;
  min-height: 100vh;
  max-width: 480px;
  background: #ffffff;
  padding: 20px 16px calc(76px + env(safe-area-inset-bottom));
}

.page-container {
  min-height: calc(100vh - 120px);
}

.page-title {
  margin: 0 0 8px;
  font-size: 28px;
  line-height: 1.2;
}

.page-summary {
  margin: 0;
  color: #6b6b70;
  font-size: 15px;
  line-height: 1.5;
}

.bottom-nav {
  position: fixed;
  left: 50%;
  bottom: 0;
  transform: translateX(-50%);
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  width: min(480px, 100%);
  padding: 10px 12px calc(10px + env(safe-area-inset-bottom));
  gap: 8px;
  border-top: 1px solid #e6e6ea;
  background: #ffffff;
}

.nav-item {
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: #6b6b70;
  padding: 10px 8px;
  font-size: 14px;
  line-height: 1.2;
}

.nav-item-active {
  background: #eef2ff;
  color: #4338ca;
  font-weight: 600;
}
"#;

#[derive(Clone, Copy, Eq, PartialEq)]
enum AppPage {
    Today,
    Inventory,
    Catalog,
    Settings,
}

impl AppPage {
    const ALL: [Self; 4] = [Self::Today, Self::Inventory, Self::Catalog, Self::Settings];

    const fn label(self) -> &'static str {
        match self {
            Self::Today => "今日",
            Self::Inventory => "入库",
            Self::Catalog => "资料",
            Self::Settings => "设置",
        }
    }

    const fn summary(self) -> &'static str {
        match self {
            Self::Today => "查看今日冲煮安排与批次状态。",
            Self::Inventory => "录入新豆批次与基础信息。",
            Self::Catalog => "维护目录项与冲煮方案资料。",
            Self::Settings => "配置门店偏好和系统参数。",
        }
    }
}

#[component]
pub fn App() -> Element {
    let mut current_page = use_signal(|| AppPage::Today);

    rsx! {
        style { "{APP_STYLES}" }
        main { class: "app-shell",
            section { class: "page-container",
                h1 { class: "page-title", "{current_page().label()}" }
                p { class: "page-summary", "{current_page().summary()}" }
            }
        }
        nav { class: "bottom-nav",
            for page in AppPage::ALL {
                button {
                    class: if current_page() == page { "nav-item nav-item-active" } else { "nav-item" },
                    onclick: move |_| current_page.set(page),
                    "{page.label()}"
                }
            }
        }
    }
}
