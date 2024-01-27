// https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING

use url::Url;

use crate::utils::name;

#[derive(thiserror::Error, Debug)]
pub enum ConnectionStringError {
    #[error(transparent)]
    Parse(#[from] url::ParseError),

    #[error(transparent)]
    Database(#[from] name::NameError),
}

#[derive(Clone)]
pub struct ConnectionStringInner {
    username: Option<String>,
    password: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    database: Option<String>,
}

#[derive(Clone)]
pub struct ConnectionString {
    inner: ConnectionStringInner,
}

impl ConnectionString {
    pub fn set_database(&mut self, database: Option<String>) {
        self.inner.database = database;
    }
}

impl ToString for ConnectionString {
    fn to_string(&self) -> String {
        let mut url = String::from("postgresql://");

        if let Some(username) = &self.inner.username {
            url.push_str(&username);
        }

        if let Some(password) = &self.inner.password {
            url.push_str(&format!(":{}", password));
        }

        if self.inner.username.is_some() {
            url.push('@')
        }

        if let Some(host) = &self.inner.host {
            url.push_str(&host);
        }

        if let Some(port) = &self.inner.port {
            url.push_str(&format!(":{}", port));
        }

        if let Some(database) = &self.inner.database {
            url.push_str(&format!("/{}", database));
        }

        url
    }
}

impl TryFrom<&str> for ConnectionString {
    type Error = ConnectionStringError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let url = Url::parse(value)?;

        let database = url.path_segments().and_then(|mut segments| segments.next());

        if let Some(db) = database {
            name::is_valid(&db)?;
        }

        let username = url.username();
        let username = if username == "" { None } else { Some(username) };

        let inner = ConnectionStringInner {
            username: username.map(|val| val.into()),
            password: url.password().map(|val| val.into()),
            host: url.host_str().map(|val| val.into()),
            port: url.port(),
            database: database.map(|val| val.into()),
        };

        Ok(ConnectionString { inner })
    }
}

impl TryFrom<String> for ConnectionString {
    type Error = ConnectionStringError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let url = Url::parse(&value)?;

        let database = url.path_segments().and_then(|mut segments| segments.next());

        if let Some(db) = database {
            name::is_valid(&db)?;
        }

        let username = url.username();
        let username = if username == "" { None } else { Some(username) };

        let inner = ConnectionStringInner {
            username: username.map(|val| val.into()),
            password: url.password().map(|val| val.into()),
            host: url.host_str().map(|val| val.into()),
            port: url.port(),
            database: database.map(|val| val.into()),
        };

        Ok(ConnectionString { inner })
    }
}
