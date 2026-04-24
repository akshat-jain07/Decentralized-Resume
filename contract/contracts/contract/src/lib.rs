#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, contracterror,
    Address, Env, String, Vec, symbol_short,
};

// ─── Errors ──────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotAuthorized     = 1,
    CredentialExists  = 2,
    CredentialMissing = 3,
    EmployerNotFound  = 4,
    AlreadyRevoked    = 5,
    InvalidInput      = 6,
    AlreadyInit       = 7,
}

// ─── Data Types ──────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug)]
pub struct WorkCredential {
    pub credential_id: String,
    pub employer:      Address,
    pub employee:      Address,
    pub job_title:     String,
    pub description:   String,
    pub start_date:    u64,
    /// 0 = current role
    pub end_date:      u64,
    /// Ledger sequence at issuance
    pub issued_at:     u32,
    pub revoked:       bool,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct EmployerProfile {
    pub name:          String,
    pub registered_at: u32,
}

// ─── Storage Keys ────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Credential(String),
    EmployeeIndex(Address),
    Employer(Address),
    Admin,
    Initialized,
}

// ─── Contract ────────────────────────────────────────────────────────────────

#[contract]
pub struct DecentralizedResume;

#[contractimpl]
impl DecentralizedResume {

    // ── Init ──────────────────────────────────────────────────────────────

    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(Error::AlreadyInit);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Initialized, &true);
        Ok(())
    }

    // ── Employer ──────────────────────────────────────────────────────────

    pub fn register_employer(
        env:      Env,
        employer: Address,
        name:     String,
    ) -> Result<(), Error> {
        employer.require_auth();
        if name.len() == 0 {
            return Err(Error::InvalidInput);
        }
        let profile = EmployerProfile {
            name,
            registered_at: env.ledger().sequence(),
        };
        env.storage()
            .persistent()
            .set(&DataKey::Employer(employer), &profile);
        Ok(())
    }

    pub fn get_employer(env: Env, employer: Address) -> Option<EmployerProfile> {
        env.storage()
            .persistent()
            .get(&DataKey::Employer(employer))
    }

    // ── Issue ─────────────────────────────────────────────────────────────

    pub fn issue_credential(
        env:           Env,
        employer:      Address,
        employee:      Address,
        credential_id: String,
        job_title:     String,
        description:   String,
        start_date:    u64,
        end_date:      u64,
    ) -> Result<(), Error> {
        employer.require_auth();

        if !env.storage().persistent().has(&DataKey::Employer(employer.clone())) {
            return Err(Error::EmployerNotFound);
        }
        if env.storage().persistent().has(&DataKey::Credential(credential_id.clone())) {
            return Err(Error::CredentialExists);
        }
        if credential_id.len() == 0 || job_title.len() == 0 {
            return Err(Error::InvalidInput);
        }

        let credential = WorkCredential {
            credential_id: credential_id.clone(),
            employer:      employer.clone(),
            employee:      employee.clone(),
            job_title,
            description,
            start_date,
            end_date,
            issued_at: env.ledger().sequence(),
            revoked:   false,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Credential(credential_id.clone()), &credential);

        // Append to employee index
        let mut ids: Vec<String> = env
            .storage()
            .persistent()
            .get(&DataKey::EmployeeIndex(employee.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        ids.push_back(credential_id);
        env.storage()
            .persistent()
            .set(&DataKey::EmployeeIndex(employee.clone()), &ids);

        env.events().publish(
            (symbol_short!("issued"), employer),
            employee,
        );

        Ok(())
    }

    // ── Revoke ────────────────────────────────────────────────────────────

    pub fn revoke_credential(
        env:           Env,
        employer:      Address,
        credential_id: String,
    ) -> Result<(), Error> {
        employer.require_auth();

        let mut cred: WorkCredential = env
            .storage()
            .persistent()
            .get(&DataKey::Credential(credential_id.clone()))
            .ok_or(Error::CredentialMissing)?;

        if cred.employer != employer {
            return Err(Error::NotAuthorized);
        }
        if cred.revoked {
            return Err(Error::AlreadyRevoked);
        }

        cred.revoked = true;
        env.storage()
            .persistent()
            .set(&DataKey::Credential(credential_id.clone()), &cred);

        env.events().publish(
            (symbol_short!("revoked"), employer),
            credential_id,
        );

        Ok(())
    }

    // ── Read ──────────────────────────────────────────────────────────────

    pub fn get_credential(env: Env, credential_id: String) -> Option<WorkCredential> {
        env.storage()
            .persistent()
            .get(&DataKey::Credential(credential_id))
    }

    pub fn get_credential_ids(env: Env, employee: Address) -> Vec<String> {
        env.storage()
            .persistent()
            .get(&DataKey::EmployeeIndex(employee))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Returns all active (non-revoked) credentials for an employee.
    pub fn get_resume(env: Env, employee: Address) -> Vec<WorkCredential> {
        let ids = Self::get_credential_ids(env.clone(), employee);
        let mut resume: Vec<WorkCredential> = Vec::new(&env);

        for id in ids.iter() {
            if let Some(cred) = env
                .storage()
                .persistent()
                .get::<DataKey, WorkCredential>(&DataKey::Credential(id.clone()))
            {
                if !cred.revoked {
                    resume.push_back(cred);
                }
            }
        }
        resume
    }

    pub fn verify_credential(env: Env, credential_id: String) -> bool {
        match env
            .storage()
            .persistent()
            .get::<DataKey, WorkCredential>(&DataKey::Credential(credential_id))
        {
            Some(cred) => !cred.revoked,
            None => false,
        }
    }

    pub fn get_admin(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotAuthorized)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    fn setup() -> (Env, Address, Address, Address) {
        let env      = Env::default();
        env.mock_all_auths();
        let contract = env.register_contract(None, DecentralizedResume);
        let admin    = Address::generate(&env);
        let employer = Address::generate(&env);
        let employee = Address::generate(&env);

        let client = DecentralizedResumeClient::new(&env, &contract);
        client.initialize(&admin);
        client.register_employer(
            &employer,
            &String::from_str(&env, "Acme Corp"),
        );
        (env, contract, employer, employee)
    }

    #[test]
    fn test_issue_and_retrieve() {
        let (env, contract, employer, employee) = setup();
        let client = DecentralizedResumeClient::new(&env, &contract);

        client.issue_credential(
            &employer,
            &employee,
            &String::from_str(&env, "cred-001"),
            &String::from_str(&env, "Software Engineer"),
            &String::from_str(&env, "Built core infra"),
            &1_700_000_000_u64,
            &0_u64,
        );

        let cred = client
            .get_credential(&String::from_str(&env, "cred-001"))
            .unwrap();

        assert_eq!(cred.job_title, String::from_str(&env, "Software Engineer"));
        assert!(!cred.revoked);
    }

    #[test]
    fn test_verify_and_revoke() {
        let (env, contract, employer, employee) = setup();
        let client = DecentralizedResumeClient::new(&env, &contract);

        client.issue_credential(
            &employer, &employee,
            &String::from_str(&env, "cred-002"),
            &String::from_str(&env, "Product Manager"),
            &String::from_str(&env, "Led roadmap"),
            &1_680_000_000_u64,
            &1_700_000_000_u64,
        );

        assert!(client.verify_credential(&String::from_str(&env, "cred-002")));
        client.revoke_credential(&employer, &String::from_str(&env, "cred-002"));
        assert!(!client.verify_credential(&String::from_str(&env, "cred-002")));
    }

    #[test]
    fn test_resume_excludes_revoked() {
        let (env, contract, employer, employee) = setup();
        let client = DecentralizedResumeClient::new(&env, &contract);

        client.issue_credential(
            &employer, &employee,
            &String::from_str(&env, "cred-a"),
            &String::from_str(&env, "Designer"),
            &String::from_str(&env, "UI work"),
            &1_660_000_000_u64, &1_680_000_000_u64,
        );
        client.issue_credential(
            &employer, &employee,
            &String::from_str(&env, "cred-b"),
            &String::from_str(&env, "Senior Designer"),
            &String::from_str(&env, "Led design system"),
            &1_680_000_000_u64, &0_u64,
        );
        client.revoke_credential(&employer, &String::from_str(&env, "cred-a"));

        let resume = client.get_resume(&employee);
        assert_eq!(resume.len(), 1);
        assert_eq!(
            resume.get(0).unwrap().credential_id,
            String::from_str(&env, "cred-b")
        );
    }
}