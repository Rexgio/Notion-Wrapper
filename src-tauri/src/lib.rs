use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;
use tauri::{
    webview::{NewWindowResponse, PageLoadEvent},
    Manager, Url, WebviewUrl, WebviewWindowBuilder,
};

const START_URL: &str = "https://app.notion.com";

static POPUP_COUNT: AtomicUsize = AtomicUsize::new(0);

fn is_notion(url: &Url) -> bool {
    url.host_str()
        .is_some_and(|h| h.ends_with("notion.so") || h.ends_with("notion.com"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();

            WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External(START_URL.parse().unwrap()),
            )
            .title("Notion")
            .inner_size(1200.0, 800.0)
            .on_new_window(move |url, features| {
                let n = POPUP_COUNT.fetch_add(1, Ordering::SeqCst);

                // Pasa a true cuando la popup visita un dominio que no es Notion (Google)
                let visited_external = Arc::new(AtomicBool::new(false));
                let flag_nav = visited_external.clone();
                let flag_load = visited_external.clone();
                let handle_load = handle.clone();

                let window = WebviewWindowBuilder::new(
                    &handle,
                    format!("popup-{n}"),
                    WebviewUrl::External("about:blank".parse().unwrap()),
                )
                .window_features(features)
                .title(url.as_str())
                .on_navigation(move |u| {
                    if u.host_str().is_some() && !is_notion(u) {
                        flag_nav.store(true, Ordering::SeqCst);
                    }
                    true
                })
                .on_page_load(move |popup, payload| {
                    if matches!(payload.event(), PageLoadEvent::Finished)
                        && is_notion(payload.url())
                        // swap evita cerrar dos veces
                        && flag_load.swap(false, Ordering::SeqCst)
                    {
                        let handle = handle_load.clone();
                        std::thread::spawn(move || {
                            // margen para que Notion reciba el login
                            std::thread::sleep(Duration::from_millis(1500));
                            let _ = popup.destroy();
                            if let Some(main) = handle.get_webview_window("main") {
                                let _ = main.navigate(START_URL.parse().unwrap());
                            }
                        });
                    }
                })
                .build()
                .unwrap();

                NewWindowResponse::Create { window }
            })
            .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error al iniciar la app");
}
