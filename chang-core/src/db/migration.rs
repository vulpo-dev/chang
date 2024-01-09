use sqlx::PgPool;

pub async fn otel(pool: &PgPool) {
    let mut migrator = sqlx::migrate!("./migrations/otel");
    migrator.set_ignore_missing(true);
    match migrator.run(pool).await {
        Ok(_) => println!("otel migrations done"),
        Err(err) => {
            println!("Failed to run otel migrations");
            panic!("{:?}", err);
        }
    };
}

pub async fn base(pool: &PgPool) {
    let mut migrator = sqlx::migrate!("./migrations/base");
    migrator.set_ignore_missing(true);
    match migrator.run(pool).await {
        Ok(_) => println!("base migrations done"),
        Err(err) => {
            println!("Failed to run base migrations");
            panic!("{:?}", err);
        }
    };
}

pub async fn events(pool: &PgPool) {
    let mut migrator = sqlx::migrate!("./migrations/events");
    migrator.set_ignore_missing(true);
    match migrator.run(pool).await {
        Ok(_) => println!("events migrations done"),
        Err(err) => {
            println!("Failed to run events migrations");
            panic!("{:?}", err);
        }
    };
}

pub async fn tasks(pool: &PgPool) {
    let mut migrator = sqlx::migrate!("./migrations/tasks");
    migrator.set_ignore_missing(true);
    match migrator.run(pool).await {
        Ok(_) => println!("tasks migrations done"),
        Err(err) => {
            println!("Failed to run tasks migrations");
            panic!("{:?}", err);
        }
    };
}
