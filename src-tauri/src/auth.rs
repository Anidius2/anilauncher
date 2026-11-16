//! Microsoft → Xbox Live → Minecraft sign-in (device code flow) and the account list.

use crate::state::AppState;
use crate::util::{now, offline_uuid, read_json, write_json, Ctx, Res};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::State;

const MS_DEVICE: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const MS_TOKEN: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const SCOPE: &str = "XboxLive.signin offline_access";

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub name: String,
    /// "msa" (licensed Microsoft account) or "offline"
    pub kind: String,
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub ms_refresh: Option<String>,
    #[serde(default)]
    pub mc_token: Option<String>,
    #[serde(default)]
    pub mc_expires: u64,
    #[serde(default)]
    pub xuid: Option<String>,
    #[serde(default)]
    pub skin_url: Option<String>,
    #[serde(default)]
    pub needs_name: bool,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AccountsFile {
    pub accounts: Vec<Account>,
    pub active: Option<String>,
}

/// What the interface sees: never the tokens.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicAccount {
    id: String,
    name: String,
    kind: String,
    uuid: String,
    skin_url: Option<String>,
    needs_name: bool,
    active: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountsView {
    accounts: Vec<PublicAccount>,
    has_license: bool,
    client_id_set: bool,
}

fn accounts_path(state: &AppState) -> std::path::PathBuf {
    state.root.join("accounts.json")
}

pub fn load(state: &AppState) -> AccountsFile {
    read_json(&accounts_path(state)).unwrap_or_default()
}

pub fn save(state: &AppState, file: &AccountsFile) -> Res<()> {
    write_json(&accounts_path(state), file)
}

fn view(state: &AppState) -> AccountsView {
    let file = load(state);
    let has_license = file.accounts.iter().any(|a| a.kind == "msa" && !a.needs_name);
    AccountsView {
        has_license,
        client_id_set: state.client_id().is_some(),
        accounts: file
            .accounts
            .iter()
            .map(|a| PublicAccount {
                id: a.id.clone(),
                name: a.name.clone(),
                kind: a.kind.clone(),
                uuid: a.uuid.clone(),
                skin_url: a.skin_url.clone(),
                needs_name: a.needs_name,
                active: file.active.as_deref() == Some(a.id.as_str()),
            })
            .collect(),
    }
}

#[tauri::command]
pub fn list_accounts(state: State<'_, AppState>) -> AccountsView {
    view(&state)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCode {
    user_code: String,
    device_code: String,
    verification_uri: String,
    interval: u64,
    expires_in: u64,
}

#[tauri::command]
pub async fn login_start(state: State<'_, AppState>) -> Res<DeviceCode> {
    let client_id = state.client_id().ok_or(
        "Не задан Client ID приложения Microsoft. Укажи его в настройках (раздел «Вход»).",
    )?;
    let resp: Value = state
        .http
        .post(MS_DEVICE)
        .form(&[("client_id", client_id.as_str()), ("scope", SCOPE)])
        .send()
        .await
        .ctx("Нет соединения с Microsoft")?
        .json()
        .await
        .ctx("Неверный ответ Microsoft")?;
    if let Some(err) = resp.get("error_description").and_then(|v| v.as_str()) {
        return Err(format!("Microsoft: {err}"));
    }
    Ok(DeviceCode {
        user_code: str_of(&resp, "user_code")?,
        device_code: str_of(&resp, "device_code")?,
        verification_uri: str_of(&resp, "verification_uri")?,
        interval: resp.get("interval").and_then(|v| v.as_u64()).unwrap_or(5),
        expires_in: resp.get("expires_in").and_then(|v| v.as_u64()).unwrap_or(900),
    })
}

fn str_of(v: &Value, key: &str) -> Res<String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("В ответе нет поля {key}"))
}

/// Waits until the user confirms the code in the browser, then finishes the whole chain.
#[tauri::command]
pub async fn login_poll(state: State<'_, AppState>, device_code: String, interval: u64) -> Res<AccountsView> {
    let client_id = state.client_id().ok_or("Не задан Client ID")?;
    let mut wait = interval.max(2);
    let deadline = now() + 900;
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
        if now() > deadline {
            return Err("Время на вход истекло. Попробуй ещё раз.".into());
        }
        let resp: Value = state
            .http
            .post(MS_TOKEN)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", client_id.as_str()),
                ("device_code", device_code.as_str()),
            ])
            .send()
            .await
            .ctx("Нет соединения с Microsoft")?
            .json()
            .await
            .ctx("Неверный ответ Microsoft")?;
        match resp.get("error").and_then(|v| v.as_str()) {
            Some("authorization_pending") => continue,
            Some("slow_down") => {
                wait += 5;
                continue;
            }
            Some("authorization_declined") => return Err("Вход отменён.".into()),
            Some("expired_token") => return Err("Код устарел. Попробуй ещё раз.".into()),
            Some(other) => return Err(format!("Microsoft: {other}")),
            None => {}
        }
        let access = str_of(&resp, "access_token")?;
        let refresh = str_of(&resp, "refresh_token")?;
        let _guard = state.accounts_lock.lock().await;
        let account = minecraft_chain(&state, &access, Some(refresh)).await?;
        let mut file = load(&state);
        file.accounts.retain(|a| !(a.kind == "msa" && a.uuid == account.uuid && !a.uuid.is_empty()));
        file.active = Some(account.id.clone());
        file.accounts.push(account);
        save(&state, &file)?;
        return Ok(view(&state));
    }
}

/// Microsoft access token → Xbox Live → XSTS → Minecraft token → licence + profile.
async fn minecraft_chain(state: &AppState, ms_access: &str, ms_refresh: Option<String>) -> Res<Account> {
    let http = &state.http;
    let xbl: Value = http
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&json!({
            "Properties": {"AuthMethod": "RPS", "SiteName": "user.auth.xboxlive.com", "RpsTicket": format!("d={ms_access}")},
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }))
        .send()
        .await
        .ctx("Нет соединения с Xbox Live")?
        .json()
        .await
        .ctx("Неверный ответ Xbox Live")?;
    let xbl_token = str_of(&xbl, "Token")?;
    let uhs = xbl
        .pointer("/DisplayClaims/xui/0/uhs")
        .and_then(|v| v.as_str())
        .ok_or("Xbox Live не вернул идентификатор пользователя")?
        .to_string();

    let xsts_resp = http
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&json!({
            "Properties": {"SandboxId": "RETAIL", "UserTokens": [xbl_token]},
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }))
        .send()
        .await
        .ctx("Нет соединения с Xbox Live")?;
    let xsts_status = xsts_resp.status();
    let xsts: Value = xsts_resp.json().await.ctx("Неверный ответ Xbox Live")?;
    if !xsts_status.is_success() {
        let code = xsts.get("XErr").and_then(|v| v.as_u64()).unwrap_or(0);
        return Err(match code {
            2148916233 => "У этого аккаунта Microsoft нет профиля Xbox. Зайди один раз на xbox.com и создай его.".into(),
            2148916235 => "Xbox Live недоступен в твоей стране.".into(),
            2148916236 | 2148916237 => "Аккаунту нужна проверка возраста на xbox.com.".into(),
            2148916238 => "Это детский аккаунт: его нужно добавить в семью Microsoft.".into(),
            _ => format!("Xbox Live отказал во входе (код {code})."),
        });
    }
    let xsts_token = str_of(&xsts, "Token")?;
    let xuid = xsts
        .pointer("/DisplayClaims/xui/0/xid")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mc_resp = http
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&json!({"identityToken": format!("XBL3.0 x={uhs};{xsts_token}")}))
        .send()
        .await
        .ctx("Нет соединения с серверами Minecraft")?;
    if mc_resp.status().as_u16() == 403 {
        return Err("Mojang пока не одобрил это приложение для входа. Нужна заявка на доступ к API Minecraft (см. инструкцию).".into());
    }
    if !mc_resp.status().is_success() {
        return Err(format!("Сервер Minecraft ответил {}", mc_resp.status()));
    }
    let mc: Value = mc_resp.json().await.ctx("Неверный ответ Minecraft")?;
    let mc_token = str_of(&mc, "access_token")?;
    let expires_in = mc.get("expires_in").and_then(|v| v.as_u64()).unwrap_or(86400);

    let ent: Value = http
        .get("https://api.minecraftservices.com/entitlements/mcstore")
        .bearer_auth(&mc_token)
        .send()
        .await
        .ctx("Нет соединения с серверами Minecraft")?
        .json()
        .await
        .unwrap_or(Value::Null);
    let owns = ent
        .get("items")
        .and_then(|v| v.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false);

    let prof_resp = http
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&mc_token)
        .send()
        .await
        .ctx("Нет соединения с серверами Minecraft")?;
    let mut account = Account {
        id: uuid::Uuid::new_v4().simple().to_string(),
        kind: "msa".into(),
        ms_refresh,
        mc_token: Some(mc_token),
        mc_expires: now() + expires_in,
        xuid,
        ..Default::default()
    };
    if prof_resp.status().as_u16() == 404 {
        if !owns {
            return Err("На этом аккаунте нет Minecraft: Java Edition.".into());
        }
        account.needs_name = true;
        account.name = "—".into();
        return Ok(account);
    }
    let prof: Value = prof_resp.json().await.ctx("Неверный профиль Minecraft")?;
    account.uuid = str_of(&prof, "id")?;
    account.name = str_of(&prof, "name")?;
    account.skin_url = skin_of(&prof);
    Ok(account)
}

fn skin_of(prof: &Value) -> Option<String> {
    prof.get("skins")?
        .as_array()?
        .iter()
        .find(|s| s.get("state").and_then(|v| v.as_str()) == Some("ACTIVE"))
        .and_then(|s| s.get("url"))
        .and_then(|v| v.as_str())
        .map(|s| s.replace("http://", "https://"))
}

/// Makes sure a licensed account has a fresh Minecraft token. Returns the updated account.
pub async fn ensure_fresh(state: &AppState, account_id: &str) -> Res<Account> {
    let _guard = state.accounts_lock.lock().await;
    let mut file = load(state);
    let idx = file
        .accounts
        .iter()
        .position(|a| a.id == account_id)
        .ok_or("Аккаунт не найден")?;
    let acc = file.accounts[idx].clone();
    if acc.kind != "msa" || acc.mc_expires > now() + 600 {
        return Ok(acc);
    }
    let refresh = acc.ms_refresh.clone().ok_or("Нужно войти заново")?;
    let client_id = state.client_id().ok_or("Не задан Client ID")?;
    let resp: Value = state
        .http
        .post(MS_TOKEN)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", client_id.as_str()),
            ("refresh_token", refresh.as_str()),
            ("scope", SCOPE),
        ])
        .send()
        .await
        .ctx("Нет соединения с Microsoft")?
        .json()
        .await
        .ctx("Неверный ответ Microsoft")?;
    if resp.get("error").is_some() {
        return Err("Сессия истекла — войди в аккаунт заново.".into());
    }
    let access = str_of(&resp, "access_token")?;
    let new_refresh = resp
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or(acc.ms_refresh.clone());
    let mut fresh = minecraft_chain(state, &access, new_refresh).await?;
    fresh.id = acc.id.clone();
    file.accounts[idx] = fresh.clone();
    save(state, &file)?;
    Ok(fresh)
}

#[tauri::command]
pub async fn name_available(state: State<'_, AppState>, account_id: String, name: String) -> Res<bool> {
    let acc = ensure_fresh(&state, &account_id).await?;
    let token = acc.mc_token.ok_or("Нет токена")?;
    let resp: Value = state
        .http
        .get(format!("https://api.minecraftservices.com/minecraft/profile/name/{name}/available"))
        .bearer_auth(token)
        .send()
        .await
        .ctx("Нет соединения")?
        .json()
        .await
        .ctx("Неверный ответ")?;
    Ok(resp.get("status").and_then(|v| v.as_str()) == Some("AVAILABLE"))
}

/// Creates the Java profile (first nickname) or changes the nickname (once per 30 days).
#[tauri::command]
pub async fn set_minecraft_name(state: State<'_, AppState>, account_id: String, name: String) -> Res<AccountsView> {
    let valid = (3..=16).contains(&name.len())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !valid {
        return Err("Ник: 3–16 символов, только латиница, цифры и _".into());
    }
    let acc = ensure_fresh(&state, &account_id).await?;
    let token = acc.mc_token.clone().ok_or("Нет токена")?;
    let resp = if acc.needs_name {
        state
            .http
            .post("https://api.minecraftservices.com/minecraft/profile")
            .bearer_auth(&token)
            .json(&json!({"profileName": name}))
            .send()
            .await
    } else {
        state
            .http
            .put(format!("https://api.minecraftservices.com/minecraft/profile/name/{name}"))
            .bearer_auth(&token)
            .send()
            .await
    }
    .ctx("Нет соединения")?;
    match resp.status().as_u16() {
        200 => {}
        400 => return Err("Такой ник нельзя использовать.".into()),
        403 => return Err("Сменить ник пока нельзя: Mojang разрешает это раз в 30 дней.".into()),
        409 | 422 => return Err("Этот ник уже занят.".into()),
        429 => return Err("Слишком много попыток, подожди немного.".into()),
        code => return Err(format!("Сервер Minecraft ответил {code}")),
    }
    let prof: Value = resp.json().await.ctx("Неверный ответ")?;
    let _guard = state.accounts_lock.lock().await;
    let mut file = load(&state);
    if let Some(a) = file.accounts.iter_mut().find(|a| a.id == account_id) {
        a.name = str_of(&prof, "name").unwrap_or(name);
        a.uuid = str_of(&prof, "id").unwrap_or_default();
        a.skin_url = skin_of(&prof);
        a.needs_name = false;
    }
    save(&state, &file)?;
    Ok(view(&state))
}

/// Offline profiles are only for people who already signed in with a licensed account.
#[tauri::command]
pub async fn add_offline(state: State<'_, AppState>, name: String) -> Res<AccountsView> {
    let valid = (3..=16).contains(&name.len())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !valid {
        return Err("Ник: 3–16 символов, только латиница, цифры и _".into());
    }
    let _guard = state.accounts_lock.lock().await;
    let mut file = load(&state);
    let licensed = file.accounts.iter().any(|a| a.kind == "msa" && !a.needs_name);
    if !licensed {
        return Err("Офлайн-профили открываются после входа лицензионным аккаунтом.".into());
    }
    let account = Account {
        id: uuid::Uuid::new_v4().simple().to_string(),
        name: name.clone(),
        kind: "offline".into(),
        uuid: offline_uuid(&name),
        ..Default::default()
    };
    file.active = Some(account.id.clone());
    file.accounts.push(account);
    save(&state, &file)?;
    Ok(view(&state))
}

#[tauri::command]
pub async fn set_active_account(state: State<'_, AppState>, account_id: String) -> Res<AccountsView> {
    let _guard = state.accounts_lock.lock().await;
    let mut file = load(&state);
    if file.accounts.iter().any(|a| a.id == account_id) {
        file.active = Some(account_id);
        save(&state, &file)?;
    }
    Ok(view(&state))
}

#[tauri::command]
pub async fn remove_account(state: State<'_, AppState>, account_id: String) -> Res<AccountsView> {
    let _guard = state.accounts_lock.lock().await;
    let mut file = load(&state);
    file.accounts.retain(|a| a.id != account_id);
    // Without a licensed account, offline profiles go too.
    if !file.accounts.iter().any(|a| a.kind == "msa") {
        file.accounts.clear();
    }
    if file.active.as_deref() == Some(account_id.as_str()) || file.accounts.is_empty() {
        file.active = file.accounts.first().map(|a| a.id.clone());
    }
    save(&state, &file)?;
    Ok(view(&state))
}
