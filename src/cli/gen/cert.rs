use color_eyre::eyre::Result;

use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

use rcgen::{BasicConstraints::*, GeneralSubtree::DnsName, *};

#[derive(clap::Args, Debug)]
/// Generate CA certificate and gey for usage with the server.
pub(crate) struct Command {
    #[arg(long, default_value = "dolores.crt")]
    /// Filename for certificate file
    cert: PathBuf,
    #[arg(long, default_value = "dolores.key")]
    /// Filename for key file
    key: PathBuf,
    #[arg(long = "domain", default_value = "localhost")]
    /// Domains that will be supported by given certificate
    domains: Vec<String>,
}

impl Command {
    pub(crate) fn run(self) -> Result<()> {
        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, "Dolores localhost certificate");
        let subtrees = self
            .domains
            .iter()
            .map(|domain| format!(".{}", domain))
            .map(DnsName)
            .collect();
        let name_constraints = NameConstraints {
            permitted_subtrees: subtrees,
            excluded_subtrees: vec![],
        };

        let mut params = CertificateParams::new(self.domains);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign];
        params.is_ca = IsCa::Ca(Constrained(0));
        params.distinguished_name = distinguished_name;
        params.name_constraints = Some(name_constraints);

        let cert = Certificate::from_params(params)?;
        let cert_pem = cert.serialize_pem()?;
        let key_pem = cert.serialize_private_key_pem();

        File::create(self.cert)?.write_all(cert_pem.as_bytes())?;
        File::create(self.key)?.write_all(key_pem.as_bytes())?;

        Ok(())
    }
}
