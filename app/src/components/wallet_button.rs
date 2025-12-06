use dioxus::prelude::*;
use futures::StreamExt;
use crate::WalletState;

#[derive(Clone)]
enum WalletAction {
    Connect,
}

#[component]
pub fn WalletButton() -> Element {
    let mut wallet = use_context::<Signal<WalletState>>();

    // Use coroutine for lifecycle-safe async operations
    let wallet_coro = use_coroutine(move |mut rx: UnboundedReceiver<WalletAction>| {
        async move {
            while let Some(action) = rx.next().await {
                match action {
                    WalletAction::Connect => {
                        #[cfg(feature = "web")]
                        {
                            match connect_phantom().await {
                                Ok(pubkey) => {
                                    wallet.write().connected = true;
                                    wallet.write().pubkey = Some(pubkey);
                                }
                                Err(e) => {
                                    tracing::error!("Wallet connection failed: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    let connect_wallet = move |_| {
        wallet_coro.send(WalletAction::Connect);
    };

    let disconnect_wallet = move |_| {
        wallet.write().connected = false;
        wallet.write().pubkey = None;
    };

    let wallet_read = wallet.read();

    if wallet_read.connected {
        let pubkey = wallet_read.pubkey.clone().unwrap_or_default();
        let short_pubkey = if pubkey.len() > 8 {
            format!("{}...{}", &pubkey[..4], &pubkey[pubkey.len()-4..])
        } else {
            pubkey.clone()
        };

        rsx! {
            div { class: "flex items-center space-x-2",
                span { class: "text-sm text-gray-400 font-mono", "{short_pubkey}" }
                button {
                    class: "btn btn-secondary text-sm",
                    onclick: disconnect_wallet,
                    "Disconnect"
                }
            }
        }
    } else {
        rsx! {
            button {
                class: "btn btn-primary",
                onclick: connect_wallet,
                "Connect Wallet"
            }
        }
    }
}

#[cfg(feature = "web")]
async fn connect_phantom() -> Result<String, String> {
    use wasm_bindgen::prelude::*;
    use js_sys::{Object, Reflect, Promise};

    let window = web_sys::window().ok_or("No window")?;

    // Check if Phantom is installed
    let solana = Reflect::get(&window, &JsValue::from_str("solana"))
        .map_err(|_| "Phantom not found")?;

    if solana.is_undefined() {
        // Open Phantom install page
        let _ = window.open_with_url("https://phantom.app/");
        return Err("Phantom not installed. Please install it and refresh.".to_string());
    }

    // Check if it's Phantom
    let is_phantom = Reflect::get(&solana, &JsValue::from_str("isPhantom"))
        .map_err(|_| "Not Phantom")?;

    if !is_phantom.as_bool().unwrap_or(false) {
        return Err("Please use Phantom wallet".to_string());
    }

    // Request connection
    let connect_fn = Reflect::get(&solana, &JsValue::from_str("connect"))
        .map_err(|_| "No connect method")?;

    let connect_fn: js_sys::Function = connect_fn.dyn_into()
        .map_err(|_| "connect is not a function")?;

    let promise = connect_fn.call0(&solana)
        .map_err(|e| format!("Connect call failed: {:?}", e))?;

    let promise: Promise = promise.dyn_into()
        .map_err(|_| "Not a promise")?;

    let result = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| format!("Connection rejected: {:?}", e))?;

    // Get public key
    let public_key = Reflect::get(&result, &JsValue::from_str("publicKey"))
        .map_err(|_| "No publicKey in response")?;

    let to_string_fn = Reflect::get(&public_key, &JsValue::from_str("toString"))
        .map_err(|_| "No toString method")?;

    let to_string_fn: js_sys::Function = to_string_fn.dyn_into()
        .map_err(|_| "toString is not a function")?;

    let pubkey_str = to_string_fn.call0(&public_key)
        .map_err(|e| format!("toString failed: {:?}", e))?;

    pubkey_str.as_string().ok_or("Public key not a string".to_string())
}

/// Sign and send a transaction via Phantom
#[cfg(feature = "web")]
pub async fn sign_and_send_transaction(tx_base64: &str) -> Result<String, String> {
    use wasm_bindgen::prelude::*;
    use js_sys::{Object, Reflect, Promise, Uint8Array};

    let window = web_sys::window().ok_or("No window")?;

    let solana = Reflect::get(&window, &JsValue::from_str("solana"))
        .map_err(|_| "Phantom not found")?;

    if solana.is_undefined() {
        return Err("Phantom not connected".to_string());
    }

    // Decode base64 transaction
    let tx_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        tx_base64,
    ).map_err(|e| format!("Invalid base64: {}", e))?;

    // Create Uint8Array from bytes
    let tx_array = Uint8Array::new_with_length(tx_bytes.len() as u32);
    tx_array.copy_from(&tx_bytes);

    // Call signAndSendTransaction
    let sign_fn = Reflect::get(&solana, &JsValue::from_str("signAndSendTransaction"))
        .map_err(|_| "No signAndSendTransaction method")?;

    let sign_fn: js_sys::Function = sign_fn.dyn_into()
        .map_err(|_| "signAndSendTransaction is not a function")?;

    // Create transaction object
    let tx_obj = Object::new();
    Reflect::set(&tx_obj, &JsValue::from_str("serialize"), &tx_array.into())
        .map_err(|_| "Failed to set serialize")?;

    let promise = sign_fn.call1(&solana, &tx_obj.into())
        .map_err(|e| format!("Sign call failed: {:?}", e))?;

    let promise: Promise = promise.dyn_into()
        .map_err(|_| "Not a promise")?;

    let result = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| format!("Signing rejected: {:?}", e))?;

    // Get signature
    let signature = Reflect::get(&result, &JsValue::from_str("signature"))
        .map_err(|_| "No signature in response")?;

    signature.as_string().ok_or("Signature not a string".to_string())
}

#[cfg(not(feature = "web"))]
async fn connect_phantom() -> Result<String, String> {
    Err("Phantom wallet only available in web mode".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn sign_and_send_transaction(_tx_base64: &str) -> Result<String, String> {
    Err("Transaction signing only available in web mode".to_string())
}
