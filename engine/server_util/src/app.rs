use actix_web::web::ServiceConfig;
pub use include_dir::include_dir;
use include_dir::Dir;

/// Registers services that serve static files. Static files are served from the filesystem in debug
/// mode and embedded in the binary in release mode.
///
/// Game client files are assumed to be located in: ../js/public/ relative to the game server.
pub fn static_files(#[allow(unused)] client_dir: &'static Dir) -> impl Fn(&mut ServiceConfig) {
    move |cfg: &mut ServiceConfig| {
        // Allows changing without recompilation.
        #[cfg(debug_assertions)]
        {
            use actix_files as fs;
            cfg.service(fs::Files::new("/admin", "../engine/js/public/").index_file("index.html"))
                .service(fs::Files::new("/", "../js/public/").index_file("index.html"));
        }

        // Allows single-binary distribution.
        #[cfg(not(debug_assertions))]
        {
            use actix_plus_static_files::{build_hashmap_from_included_dir, ResourceFiles};
            cfg.service(ResourceFiles::new(
                "/admin",
                build_hashmap_from_included_dir(&include_dir!("../js/public/")),
            ))
            .service(ResourceFiles::new(
                "/*",
                build_hashmap_from_included_dir(client_dir),
            ));
        }
    }
}
