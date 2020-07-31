use actix_web::web::{self, ServiceConfig};

use crate::router;

pub(crate) fn conf_admin(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/admin")
            .service(web::resource("/user").route(web::post().to(router::admin::update_user)))
            .service(web::resource("/post").route(web::post().to(router::admin::update_post)))
            .service(web::resource("/topic").route(web::post().to(router::admin::update_topic)))
            .service(
                web::scope("/category")
                    .service(
                        web::resource("/remove/{category_id}")
                            .route(web::get().to(router::admin::remove_category)),
                    )
                    .service(
                        web::resource("/update")
                            .route(web::post().to(router::admin::update_category)),
                    )
                    .service(web::resource("").route(web::post().to(router::admin::add_category))),
            ),
    );
}

pub(crate) fn conf_auth(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .service(web::resource("/register").route(web::post().to(router::auth::register)))
            .service(web::resource("/login").route(web::post().to(router::auth::login)))
            .service(
                web::resource("/activation/mail")
                    .route(web::post().to(router::auth::add_activation_mail)),
            )
            .service(
                web::resource("/activation/mail/{uuid}")
                    .route(web::get().to(router::auth::activate_by_mail)),
            ),
    );
}

pub(crate) fn conf_psn(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/psn")
            .service(
                web::resource("/auth").route(web::get().to(router::psn::query_handler_with_jwt)),
            )
            .service(web::resource("/community").route(web::get().to(router::psn::community)))
            .service(web::resource("").route(web::get().to(router::psn::query_handler))),
    );
}

pub(crate) fn conf_test(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/test")
            .service(web::resource("/raw").route(web::get().to(router::test::raw)))
            .service(web::resource("/raw_cache").route(web::get().to(router::test::raw_cache)))
            .service(web::resource("/topic").route(web::get().to(router::test::add_topic)))
            .service(web::resource("/post").route(web::get().to(router::test::add_post))),
    );
}

pub(crate) fn conf_comm(cfg: &mut ServiceConfig) {
    cfg.service(web::resource("/categories").route(web::get().to(router::category::query_handler)))
        .service(
            web::scope("/post")
                .service(web::resource("/update").route(web::post().to(router::post::update)))
                .service(web::resource("/{pid}").route(web::get().to(router::post::get)))
                .service(web::resource("").route(web::post().to(router::post::add))),
        )
        .service(
            web::scope("/topic")
                .service(web::resource("/update").route(web::post().to(router::topic::update)))
                .service(
                    web::resource("")
                        .route(web::get().to(router::topic::query_handler))
                        .route(web::post().to(router::topic::add)),
                ),
        )
        .service(
            web::scope("/user")
                .service(web::resource("/update").route(web::post().to(router::user::update)))
                .service(web::resource("/{id}").route(web::get().to(router::user::get))),
        );
}
