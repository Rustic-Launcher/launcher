//! Wire types of the Rustic Cloud API.
//!
//! These structs are the single source of truth for the JSON bodies: the server derives the
//! OpenAPI schemas from them (feature `openapi`) and the generated client methods
//! (`generated.rs`) use them directly. Schema names in `openapi.json` equal the Rust type names.

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

macro_rules! wire {
    ($($item:item)*) => { $(
        #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
        #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
        $item
    )* };
}

// ---------------------------------------------------------------- common

wire! {
    /// Error envelope returned with every non-2xx status.
    pub struct ErrorBody {
        pub error: ErrorDetail,
    }

    pub struct ErrorDetail {
        /// Stable machine-readable code (`invalid_input`, `unauthorized`, `rate_limited`...).
        pub code: String,
        /// Human readable message (English; the UI translates by `code`).
        pub message: String,
    }

    pub struct Health {
        pub status: String,
        pub version: String,
    }

    /// A user as seen by other users (no email, no date of birth).
    pub struct PublicUser {
        pub id: Uuid,
        pub username: String,
        pub display_name: String,
    }
}

// ---------------------------------------------------------------- auth

wire! {
    #[derive(Copy, Eq, Hash, Default)]
    #[serde(rename_all = "snake_case")]
    /// `web` receives the refresh token in an HttpOnly cookie instead of the body.
    pub enum ClientKind {
        #[default]
        Launcher,
        Web,
    }

    pub struct SignupRequest {
        pub email: String,
        pub password: String,
        /// 3-20 chars, `[a-zA-Z0-9_]`.
        pub username: String,
        pub date_of_birth: NaiveDate,
        /// ISO 3166-1 alpha-2 country code (age rules depend on it).
        pub country: String,
        /// Version of the privacy policy the user accepted at signup.
        pub privacy_policy_version: String,
        #[serde(default)]
        pub locale: Option<String>,
    }

    pub struct SignupResponse {
        pub user_id: Uuid,
        pub email_verification_required: bool,
        /// True when the user is under the digital-consent age of their country: features are
        /// restricted until a guardian links the account.
        pub guardian_required: bool,
    }

    pub struct VerifyEmailRequest {
        pub token: String,
    }

    pub struct EmailRequest {
        pub email: String,
    }

    pub struct LoginRequest {
        pub email: String,
        pub password: String,
        #[serde(default)]
        pub device_name: Option<String>,
        #[serde(default)]
        pub client: ClientKind,
    }

    #[derive(Copy, Eq)]
    #[serde(rename_all = "snake_case")]
    pub enum LoginStatus {
        Ok,
        MfaRequired,
        /// Microsoft sign-in of an unknown identity: finish with `/v1/auth/microsoft/complete`.
        SignupRequired,
    }

    pub struct LoginResponse {
        pub status: LoginStatus,
        #[serde(default)]
        pub tokens: Option<TokenPair>,
        /// Present when `status = mfa_required`: send it to `/v1/auth/login/mfa` (5 min).
        #[serde(default)]
        pub mfa_token: Option<String>,
        /// Present when `status = signup_required` (Microsoft sign-in): 10 min, single use.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub signup_token: Option<String>,
        /// The Microsoft account's email, when `status = signup_required`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub email: Option<String>,
    }

    pub struct TokenPair {
        pub token_type: String,
        /// Short-lived JWT (15 min) for `Authorization: Bearer` and the realtime socket.
        pub access_token: String,
        pub access_expires_at: DateTime<Utc>,
        /// Opaque rotating refresh token (30 days). Absent for `client = web` (cookie instead).
        #[serde(default)]
        pub refresh_token: Option<String>,
        pub session_id: Uuid,
    }

    pub struct MfaLoginRequest {
        pub mfa_token: String,
        #[serde(default)]
        pub code: Option<String>,
        #[serde(default)]
        pub recovery_code: Option<String>,
        #[serde(default)]
        pub device_name: Option<String>,
        #[serde(default)]
        pub client: ClientKind,
    }

    pub struct RefreshRequest {
        /// Required for the launcher; the web app sends the cookie instead.
        #[serde(default)]
        pub refresh_token: Option<String>,
        #[serde(default)]
        pub client: ClientKind,
    }

    pub struct ResetPasswordRequest {
        pub token: String,
        pub new_password: String,
    }

    pub struct ChangePasswordRequest {
        pub current_password: String,
        pub new_password: String,
    }

    /// "Continue with Microsoft" (ADR 0090): the id_token the website got from Microsoft
    /// (OIDC code flow + PKCE) and the nonce it generated.
    pub struct MicrosoftSignInRequest {
        pub id_token: String,
        pub nonce: String,
        #[serde(default)]
        pub client: ClientKind,
        #[serde(default)]
        pub device_name: Option<String>,
    }

    /// The "complete your account" step of a Microsoft sign-up (same age gate and consent as
    /// an email sign-up).
    pub struct MicrosoftCompleteRequest {
        pub signup_token: String,
        pub username: String,
        pub date_of_birth: NaiveDate,
        pub country: String,
        pub privacy_policy_version: String,
        #[serde(default)]
        pub locale: Option<String>,
        #[serde(default)]
        pub client: ClientKind,
        #[serde(default)]
        pub device_name: Option<String>,
    }

    pub struct MicrosoftLinkRequest {
        pub id_token: String,
        pub nonce: String,
    }

    /// A sign-in method linked to the account (`microsoft`).
    pub struct ExternalIdentity {
        pub provider: String,
        pub email: Option<String>,
        pub linked_at: DateTime<Utc>,
    }

    pub struct IdentitiesResponse {
        /// Whether the account can sign in with an email and password.
        pub has_password: bool,
        pub identities: Vec<ExternalIdentity>,
    }

    /// First password of an account created with Microsoft.
    pub struct SetPasswordRequest {
        pub new_password: String,
    }

    /// Single-use code (60 s) the website hands to the launcher through `rustic://auth?code=`.
    pub struct HandoffResponse {
        pub code: String,
        pub expires_at: DateTime<Utc>,
    }

    pub struct HandoffExchangeRequest {
        pub code: String,
        #[serde(default)]
        pub device_name: Option<String>,
        #[serde(default)]
        pub client: ClientKind,
    }

    pub struct SessionInfo {
        pub id: Uuid,
        pub device_name: Option<String>,
        pub user_agent: Option<String>,
        pub ip: Option<String>,
        pub created_at: DateTime<Utc>,
        pub last_seen_at: DateTime<Utc>,
        pub current: bool,
    }

    pub struct TotpEnrollResponse {
        /// Base32 secret (show once, or render `otpauth_url` as a QR code client-side).
        pub secret: String,
        pub otpauth_url: String,
    }

    pub struct TotpCodeRequest {
        pub code: String,
    }

    pub struct RecoveryCodesResponse {
        /// Ten single-use codes, shown once.
        pub codes: Vec<String>,
    }

    pub struct ReauthRequest {
        #[serde(default)]
        pub password: Option<String>,
        #[serde(default)]
        pub totp_code: Option<String>,
    }
}

// ---------------------------------------------------------------- me, consent, parental

wire! {
    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum Role {
        User,
        Moderator,
        Admin,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum ChatMode {
        Everyone,
        FriendsOnly,
        Disabled,
    }

    /// What the account may do right now (age rules + parental controls + bans).
    pub struct Capabilities {
        pub is_minor: bool,
        /// Under the digital-consent age and no guardian linked yet.
        pub needs_guardian: bool,
        pub can_chat: bool,
        pub chat_mode: ChatMode,
        pub can_purchase: bool,
        pub purchases_need_guardian_approval: bool,
        pub friend_requests_allowed: bool,
        pub chat_banned: bool,
        /// May appear in (and see) friend suggestions of players met (ADR 0145): adults only.
        #[serde(default)]
        pub discoverable_allowed: bool,
    }

    pub struct Me {
        pub id: Uuid,
        pub email: Option<String>,
        pub email_verified: bool,
        pub username: String,
        pub display_name: String,
        pub date_of_birth: Option<NaiveDate>,
        pub country: Option<String>,
        pub locale: Option<String>,
        pub role: Role,
        pub totp_enabled: bool,
        pub created_at: DateTime<Utc>,
        pub deletion_scheduled_for: Option<DateTime<Utc>>,
        pub capabilities: Capabilities,
        /// "Appear in suggestions of players I meet" as in effect: the setting (default on) and
        /// `capabilities.discoverable_allowed` (always off for minors). ADR 0145.
        #[serde(default)]
        pub discoverable: bool,
    }

    pub struct UpdateMeRequest {
        #[serde(default)]
        pub display_name: Option<String>,
        #[serde(default)]
        pub locale: Option<String>,
        #[serde(default)]
        pub country: Option<String>,
        /// Only accepted while unset (accounts created with Microsoft).
        #[serde(default)]
        pub date_of_birth: Option<NaiveDate>,
        /// Friend suggestions opt-out (ADR 0145). `true` is refused (`discoverable_not_allowed`)
        /// for minors; `false` also deletes the recorded sightings at once.
        #[serde(default)]
        pub discoverable: Option<bool>,
    }

    pub struct ConsentState {
        /// Telemetry is off by default.
        pub telemetry: bool,
        pub telemetry_updated_at: Option<DateTime<Utc>>,
        pub privacy_policy_version: Option<String>,
        pub privacy_policy_accepted_at: Option<DateTime<Utc>>,
        pub current_privacy_policy_version: String,
    }

    pub struct TelemetryConsentRequest {
        pub granted: bool,
    }

    pub struct PrivacyAcceptRequest {
        pub version: String,
    }

    pub struct GuardianInviteResponse {
        /// Give this code to the guardian; valid 7 days, single use.
        pub code: String,
        pub expires_at: DateTime<Utc>,
    }

    pub struct GuardianLinkRequest {
        pub code: String,
    }

    pub struct ParentalControls {
        pub purchases_require_approval: bool,
        /// Monthly cap in minor units of `spend_limit_currency`; `null` = no cap.
        pub monthly_spend_limit_minor: Option<i64>,
        pub spend_limit_currency: Currency,
        pub chat_mode: ChatMode,
        pub friend_requests_allowed: bool,
    }

    pub struct ChildSummary {
        pub user: PublicUser,
        pub linked_at: DateTime<Utc>,
        pub controls: ParentalControls,
    }

    pub struct GuardianInfo {
        pub guardian: PublicUser,
        pub linked_at: DateTime<Utc>,
    }

    #[derive(Copy, Eq)]
    #[serde(rename_all = "snake_case")]
    pub enum ApprovalStatus {
        Pending,
        Approved,
        Denied,
    }

    pub struct PurchaseApproval {
        pub id: Uuid,
        pub child: PublicUser,
        pub order_id: Uuid,
        pub currency: Currency,
        pub total_minor: i64,
        pub items: Vec<String>,
        pub status: ApprovalStatus,
        pub created_at: DateTime<Utc>,
    }

    pub struct ApprovalDecision {
        pub approve: bool,
    }
}

// ---------------------------------------------------------------- minecraft link

wire! {
    /// Step 1 of the link (ADR 0011): a one-time `server_id`. The launcher then calls Mojang's
    /// session server `POST /session/minecraft/join` with its own Minecraft token, the profile
    /// UUID and this `server_id`. The token goes to Mojang only, never to Rustic Cloud.
    pub struct MinecraftLinkChallenge {
        pub server_id: String,
        pub expires_at: DateTime<Utc>,
    }

    /// Step 2: the API asks Mojang `hasJoined(username, server_id)` and binds the returned UUID.
    pub struct MinecraftLinkFinishRequest {
        /// Current Minecraft username of the profile that joined.
        pub username: String,
    }

    pub struct MinecraftAccount {
        pub uuid: Uuid,
        pub username: String,
        pub linked_at: DateTime<Utc>,
        pub last_verified_at: DateTime<Utc>,
    }

    pub struct PublicPlayer {
        pub user: PublicUser,
        pub equipped: Vec<EquippedSlot>,
    }

    pub struct MinecraftLookup {
        pub uuid: Uuid,
        /// `null` when the Minecraft account is not linked to a Rustic account.
        pub player: Option<PublicPlayer>,
    }

    pub struct LookupBatchRequest {
        /// Up to 100 Minecraft UUIDs.
        pub uuids: Vec<Uuid>,
    }

    pub struct LookupBatchResponse {
        pub players: Vec<MinecraftLookup>,
    }
}

// ---------------------------------------------------------------- social

wire! {
    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum PresenceStatus {
        Online,
        Away,
        Busy,
        /// Appears offline to friends.
        Invisible,
        Offline,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum ActivityKind {
        InLauncher,
        Playing,
        Hosting,
    }

    pub struct Activity {
        pub kind: ActivityKind,
        #[serde(default)]
        pub instance_name: Option<String>,
        #[serde(default)]
        pub server_name: Option<String>,
        /// Only forwarded to friends when the user marked the server as shareable.
        #[serde(default)]
        pub server_address: Option<String>,
    }

    pub struct Presence {
        pub status: PresenceStatus,
        pub activity: Option<Activity>,
        pub updated_at: DateTime<Utc>,
    }

    pub struct UpdatePresenceRequest {
        pub status: PresenceStatus,
        #[serde(default)]
        pub activity: Option<Activity>,
        /// When false, `activity.server_address` is dropped before fan-out.
        #[serde(default)]
        pub share_server: bool,
    }

    pub struct Friend {
        pub user: PublicUser,
        pub since: DateTime<Utc>,
        pub presence: Presence,
    }

    pub struct FriendRequest {
        pub id: Uuid,
        pub from: PublicUser,
        pub to: PublicUser,
        pub created_at: DateTime<Utc>,
    }

    pub struct FriendRequests {
        pub incoming: Vec<FriendRequest>,
        pub outgoing: Vec<FriendRequest>,
    }

    /// One of `user_id`, `username` (Rustic) or `minecraft_name` (linked Minecraft profile).
    pub struct SendFriendRequest {
        #[serde(default)]
        pub username: Option<String>,
        #[serde(default)]
        pub minecraft_name: Option<String>,
        #[serde(default)]
        pub user_id: Option<Uuid>,
    }

    /// How the caller relates to a looked-up user.
    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum LookupRelation {
        /// A friend request can be sent.
        None,
        Friends,
        /// The caller already sent a pending request.
        OutgoingRequest,
        /// The user already sent the caller a request: accept it (`request_id`).
        IncomingRequest,
    }

    /// Exact-match result of `GET /v1/users/lookup`, shown before sending a friend request.
    pub struct UserLookup {
        pub user: PublicUser,
        /// Linked Minecraft profile name. Hidden for minors unless it is what was searched.
        pub minecraft_name: Option<String>,
        pub relation: LookupRelation,
        /// Pending incoming request id when `relation` is `incoming_request`.
        pub request_id: Option<Uuid>,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum SuggestionReason {
        /// Same party at the same time.
        Party,
        /// Same server (shared by both) at the same time.
        Server,
    }

    pub struct FriendSuggestion {
        pub user: PublicUser,
        pub minecraft_name: Option<String>,
        pub reason: SuggestionReason,
        /// The caller's own name for the server (from their presence), when `reason` is `server`.
        pub server_name: Option<String>,
        /// End of the most recent time together.
        pub seen_at: DateTime<Utc>,
    }

    /// `GET /v1/friends/suggestions` (ADR 0145). Both lists are most recent first, 10 at most.
    pub struct FriendSuggestions {
        /// `false` when the caller opted out or is a minor: nothing is recorded, lists are empty.
        pub discoverable: bool,
        /// Same party, or the same server for at least 10 minutes in total, in the last 14 days.
        pub played_with: Vec<FriendSuggestion>,
        /// Same server at overlapping times in the last 14 days (not already in `played_with`).
        pub crossed: Vec<FriendSuggestion>,
    }

    pub struct PartyMember {
        pub user: PublicUser,
        pub joined_at: DateTime<Utc>,
    }

    pub struct Party {
        pub id: Uuid,
        pub leader_id: Uuid,
        pub max_size: i32,
        pub members: Vec<PartyMember>,
        pub created_at: DateTime<Utc>,
    }

    pub struct UserIdRequest {
        pub user_id: Uuid,
    }

    pub struct PartyInvite {
        pub id: Uuid,
        pub party_id: Uuid,
        pub from: PublicUser,
        pub created_at: DateTime<Utc>,
        pub expires_at: DateTime<Utc>,
    }

    pub struct Message {
        pub id: Uuid,
        pub sender_id: Uuid,
        pub recipient_id: Uuid,
        pub body: String,
        pub created_at: DateTime<Utc>,
        /// When the recipient read it (read receipt); `null` while unread.
        pub read_at: Option<DateTime<Utc>>,
    }

    /// One direct-message conversation (most recent first in `GET /v1/conversations`).
    pub struct Conversation {
        pub peer: PublicUser,
        pub presence: Presence,
        pub last_message: Message,
        /// Messages from `peer` not read yet.
        pub unread: i64,
    }

    pub struct ReadReceipt {
        pub peer_id: Uuid,
        pub read_at: DateTime<Utc>,
        /// Messages marked read by this call.
        pub marked: i64,
    }

    pub struct SendMessageRequest {
        /// 1-2000 characters. Passed through the profanity filter hook.
        pub body: String,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum ReportReason {
        Spam,
        Harassment,
        InappropriateContent,
        Cheating,
        Impersonation,
        Other,
    }

    pub struct ReportRequest {
        pub target_user_id: Uuid,
        #[serde(default)]
        pub message_id: Option<Uuid>,
        pub reason: ReportReason,
        #[serde(default)]
        pub details: Option<String>,
    }

    pub struct ReportCreated {
        pub id: Uuid,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum InvitationKind {
        Party,
        Server,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum InvitationStatus {
        Pending,
        Accepted,
        Declined,
        Expired,
    }

    pub struct ServerJoinInfo {
        pub name: String,
        /// `host[:port]` to connect to.
        pub address: String,
        #[serde(default)]
        pub rustic_server_id: Option<Uuid>,
    }

    pub struct Invitation {
        pub id: Uuid,
        pub from: PublicUser,
        pub kind: InvitationKind,
        pub party_id: Option<Uuid>,
        pub server: Option<ServerJoinInfo>,
        pub status: InvitationStatus,
        pub created_at: DateTime<Utc>,
        pub expires_at: DateTime<Utc>,
    }

    pub struct CreateInvitationRequest {
        pub to_user_id: Uuid,
        pub server: ServerJoinInfo,
    }

    pub struct RespondInvitationRequest {
        pub accept: bool,
    }

    pub struct Notification {
        pub id: Uuid,
        /// `friend_request`, `friend_accepted`, `party_invite`, `invitation`, `message`,
        /// `purchase_approval`, `order_paid`, `order_refunded`, `moderation`...
        pub kind: String,
        #[cfg_attr(feature = "openapi", schema(value_type = Object))]
        pub payload: serde_json::Value,
        pub created_at: DateTime<Utc>,
        pub read_at: Option<DateTime<Utc>>,
    }
}

// ---------------------------------------------------------------- cosmetics & store

wire! {
    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "UPPERCASE")]
    pub enum Currency {
        Eur,
        Usd,
        Gbp,
        Chf,
    }

    /// Cosmetic slots = the cosmetics format's categories (`packages/cosmetics-format`,
    /// `Category`). Purely visual. There is deliberately no cape slot (docs/LEGAL.md).
    #[derive(Copy, Eq, Hash, PartialOrd, Ord)]
    #[serde(rename_all = "snake_case")]
    pub enum CosmeticSlot {
        /// A hat, a crown, ears.
        Head,
        /// A backpack, wings that never act like an elytra. Never a cape.
        Back,
        /// Over the body (a shirt).
        Top,
        /// A small pet sitting on a shoulder.
        ShoulderCompanion,
        /// A small creature floating beside the player.
        Companion,
        /// Particles left behind while walking.
        Trail,
        /// A see-through model around the feet or body.
        Aura,
        /// A short body animation (emote wheel).
        Emote,
        /// A badge after the name and a coloured line under it.
        Nametag,
    }

    pub struct CosmeticItem {
        pub id: Uuid,
        pub slug: String,
        pub name: String,
        pub description: String,
        pub slot: CosmeticSlot,
        /// Cosmetic id on the CDN (`cosmetics/<asset_ref>/<asset_version>/<asset_hash>.json`).
        pub asset_ref: String,
        /// Bundle version on the CDN (empty until published by the pipeline).
        pub asset_version: String,
        /// SHA-256 (hex) of the bundle: its CDN address and integrity check.
        pub asset_hash: String,
    }

    pub struct Category {
        pub id: String,
        pub name: String,
        pub sort: i32,
    }

    pub struct Price {
        pub currency: Currency,
        /// Final amount the customer pays, in minor units.
        pub amount_minor: i64,
        /// True when `amount_minor` includes VAT (EU, UK, CH display rule).
        pub tax_inclusive: bool,
        /// Pre-formatted display string, e.g. `4,99 €` / `$4.99 + tax`.
        pub display: String,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum ProductKind {
        Item,
        Bundle,
    }

    pub struct Product {
        pub id: Uuid,
        pub slug: String,
        pub kind: ProductKind,
        pub name: String,
        pub description: String,
        pub category: String,
        pub items: Vec<CosmeticItem>,
        /// The price actually charged today (the discounted one while a daily offer runs).
        pub price: Option<Price>,
        /// Only set for authenticated catalog requests.
        pub owned: Option<bool>,
        pub created_at: DateTime<Utc>,
        /// Highlighted by staff (e.g. a large card).
        pub featured: bool,
        /// Today's daily offer on this product in the requested currency, if any.
        pub offer: Option<ProductOffer>,
    }

    /// A daily offer (ADR 0091): the same for every buyer, visible in advance, no random draw.
    pub struct ProductOffer {
        pub percent_off: i32,
        /// Undiscounted price, same currency as `Product.price`.
        pub regular: Price,
        /// EU Omnibus reference (Directive 98/6/EC art. 6a): lowest price applied to the product
        /// in this currency during the 30 days before the offer day, in minor units.
        pub reference_minor: i64,
        /// When the offer ends (next UTC midnight).
        pub ends_at: DateTime<Utc>,
    }

    /// A staff-curated collection (theme, season, creator...).
    pub struct StoreCollection {
        pub id: Uuid,
        pub slug: String,
        pub name: String,
        pub description: String,
        /// `#rrggbb`.
        pub accent: String,
        /// Shown as the store's hero banner.
        pub hero: bool,
        /// In display order; only active products of this catalog response.
        pub product_ids: Vec<Uuid>,
    }

    /// Today's daily offers (one UTC day, same for everyone).
    pub struct DailyOffers {
        pub day: NaiveDate,
        /// Next UTC midnight.
        pub ends_at: DateTime<Utc>,
        pub product_ids: Vec<Uuid>,
    }

    pub struct Catalog {
        pub currency: Currency,
        pub country: Option<String>,
        pub prices_include_tax: bool,
        /// Legal display note (e.g. "Prices include VAT." / "Taxes calculated at checkout.").
        pub tax_note: String,
        pub categories: Vec<Category>,
        pub products: Vec<Product>,
        /// Active collections ordered by `sort`, then name.
        pub collections: Vec<StoreCollection>,
        /// Product ids by trending rank (best first).
        pub trending: Vec<Uuid>,
        pub daily: DailyOffers,
    }

    /// "Premium: coming soon" waitlist status of the caller.
    pub struct PremiumWaitlist {
        pub joined: bool,
        pub joined_at: Option<DateTime<Utc>>,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum AcquisitionSource {
        Purchase,
        Gift,
        Promo,
        Grant,
    }

    pub struct InventoryItem {
        pub item: CosmeticItem,
        pub acquired_at: DateTime<Utc>,
        pub source: AcquisitionSource,
    }

    pub struct EquippedSlot {
        pub slot: CosmeticSlot,
        pub item_id: Uuid,
    }

    pub struct Equipped {
        pub items: Vec<EquippedSlot>,
        pub updated_at: Option<DateTime<Utc>>,
    }

    pub struct UpdateEquippedRequest {
        pub items: Vec<EquippedSlot>,
    }

    pub struct Outfit {
        pub id: Uuid,
        pub name: String,
        pub items: Vec<EquippedSlot>,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
    }

    pub struct OutfitRequest {
        pub name: String,
        pub items: Vec<EquippedSlot>,
    }

    pub struct QuoteRequest {
        pub product_ids: Vec<Uuid>,
        pub currency: Currency,
        /// ISO country of the buyer (VAT rules).
        pub country: String,
        #[serde(default)]
        pub promo_code: Option<String>,
        #[serde(default)]
        pub creator_code: Option<String>,
        /// Spend wallet credit (redeemed gift cards) first.
        #[serde(default)]
        pub use_wallet: bool,
    }

    pub struct QuoteLine {
        pub product_id: Uuid,
        pub name: String,
        pub unit_price_minor: i64,
    }

    pub struct Quote {
        pub currency: Currency,
        pub lines: Vec<QuoteLine>,
        pub subtotal_minor: i64,
        pub discount_minor: i64,
        pub wallet_applied_minor: i64,
        pub total_minor: i64,
        pub tax_inclusive: bool,
        pub promo_applied: bool,
        pub creator_code_applied: bool,
        pub warnings: Vec<String>,
    }

    /// EU consumer law: explicit request for immediate execution and acknowledgement of the
    /// loss of the 14-day withdrawal right for digital content. Both must be `true`.
    pub struct PurchaseConsent {
        pub immediate_execution: bool,
        pub withdrawal_waiver: bool,
        /// Version of the consent wording displayed (see `GET /v1/store/legal`).
        pub wording_version: String,
    }

    pub struct CheckoutRequest {
        pub quote: QuoteRequest,
        pub consent: PurchaseConsent,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum OrderStatus {
        Pending,
        AwaitingGuardian,
        GuardianDenied,
        Paid,
        Refunded,
        Disputed,
        Cancelled,
        Failed,
    }

    pub struct OrderLine {
        pub product_id: Uuid,
        pub name: String,
        pub unit_price_minor: i64,
        /// Cosmetic items this line grants ("Equip now" after checkout).
        pub item_ids: Vec<Uuid>,
    }

    pub struct Order {
        pub id: Uuid,
        pub status: OrderStatus,
        pub currency: Currency,
        pub subtotal_minor: i64,
        pub discount_minor: i64,
        pub wallet_applied_minor: i64,
        pub total_minor: i64,
        pub lines: Vec<OrderLine>,
        pub consent_recorded_at: DateTime<Utc>,
        pub consent_wording_version: String,
        pub created_at: DateTime<Utc>,
        pub paid_at: Option<DateTime<Utc>>,
        pub invoice_url: Option<String>,
    }

    pub struct CheckoutResponse {
        pub order: Order,
        /// Stripe Checkout URL to open; `null` when fully paid by wallet or awaiting a guardian.
        pub checkout_url: Option<String>,
    }

    pub struct StoreLegal {
        pub consent_wording_version: String,
        pub immediate_execution_text: String,
        pub withdrawal_waiver_text: String,
        pub stripe_tax_enabled: bool,
    }

    pub struct WalletBalance {
        pub currency: Currency,
        pub amount_minor: i64,
    }

    pub struct Wallet {
        pub balances: Vec<WalletBalance>,
    }

    pub struct RedeemGiftCardRequest {
        pub code: String,
    }

    pub struct PriceInput {
        pub currency: Currency,
        pub amount_minor: i64,
    }

    pub struct CreateItemRequest {
        pub slug: String,
        pub name: String,
        pub description: String,
        pub slot: CosmeticSlot,
        pub asset_ref: String,
        #[serde(default)]
        pub asset_version: String,
        #[serde(default)]
        pub asset_hash: String,
    }

    pub struct CreateProductRequest {
        pub slug: String,
        pub kind: ProductKind,
        pub name: String,
        pub description: String,
        pub category: String,
        pub item_ids: Vec<Uuid>,
        pub prices: Vec<PriceInput>,
    }

    /// A product as staff see it (inactive ones included).
    pub struct AdminProduct {
        pub id: Uuid,
        pub slug: String,
        pub kind: ProductKind,
        pub name: String,
        pub description: String,
        pub category_id: String,
        pub active: bool,
        pub featured: bool,
        /// 1 = most trending; null when not trending.
        pub trending_rank: Option<i32>,
        /// Regular prices (before any daily offer).
        pub prices: Vec<PriceInput>,
        pub item_ids: Vec<Uuid>,
        pub created_at: DateTime<Utc>,
    }

    /// Every field optional; `prices` and `item_ids` replace the whole set when present.
    pub struct UpdateProductRequest {
        #[serde(default)]
        pub name: Option<String>,
        #[serde(default)]
        pub description: Option<String>,
        #[serde(default)]
        pub category_id: Option<String>,
        #[serde(default)]
        pub active: Option<bool>,
        #[serde(default)]
        pub featured: Option<bool>,
        #[serde(default)]
        pub prices: Option<Vec<PriceInput>>,
        #[serde(default)]
        pub item_ids: Option<Vec<Uuid>>,
    }

    pub struct SetTrendingRequest {
        /// Ranks 1..n in this order; every other product loses its rank.
        pub product_ids: Vec<Uuid>,
    }

    /// A collection as staff see it.
    pub struct AdminCollection {
        pub id: Uuid,
        pub slug: String,
        pub name: String,
        pub description: String,
        pub accent: String,
        pub hero: bool,
        /// In display order (inactive products included).
        pub product_ids: Vec<Uuid>,
        pub active: bool,
        pub sort: i32,
        pub created_at: DateTime<Utc>,
    }

    pub struct CreateCollectionRequest {
        /// 2-64 chars `[a-z0-9-]`, unique.
        pub slug: String,
        pub name: String,
        #[serde(default)]
        pub description: Option<String>,
        /// `#rrggbb`, default `#ff6a00`.
        #[serde(default)]
        pub accent: Option<String>,
        #[serde(default)]
        pub hero: Option<bool>,
        #[serde(default)]
        pub active: Option<bool>,
        #[serde(default)]
        pub sort: Option<i32>,
        #[serde(default)]
        pub product_ids: Option<Vec<Uuid>>,
    }

    /// Every field optional; `product_ids` replaces the list (in order) when present.
    pub struct UpdateCollectionRequest {
        #[serde(default)]
        pub slug: Option<String>,
        #[serde(default)]
        pub name: Option<String>,
        #[serde(default)]
        pub description: Option<String>,
        #[serde(default)]
        pub accent: Option<String>,
        #[serde(default)]
        pub hero: Option<bool>,
        #[serde(default)]
        pub active: Option<bool>,
        #[serde(default)]
        pub sort: Option<i32>,
        #[serde(default)]
        pub product_ids: Option<Vec<Uuid>>,
    }

    pub struct DailyOfferEntry {
        /// UTC day.
        pub day: NaiveDate,
        pub product_id: Uuid,
        pub percent_off: i32,
    }

    pub struct DailyOfferInput {
        pub product_id: Uuid,
        /// 1-90.
        pub percent_off: i32,
    }

    /// Replaces the offers of one day (at most 12).
    pub struct SetDailyOffersRequest {
        pub offers: Vec<DailyOfferInput>,
    }

    pub struct CreatePromoCodeRequest {
        pub code: String,
        #[serde(default)]
        pub percent_off: Option<i32>,
        #[serde(default)]
        pub amount_off_minor: Option<i64>,
        #[serde(default)]
        pub currency: Option<Currency>,
        #[serde(default)]
        pub max_redemptions: Option<i32>,
        #[serde(default)]
        pub valid_until: Option<DateTime<Utc>>,
    }

    pub struct PromoCode {
        pub id: Uuid,
        pub code: String,
        pub percent_off: Option<i32>,
        pub amount_off_minor: Option<i64>,
        pub currency: Option<Currency>,
        pub max_redemptions: Option<i32>,
        pub redeemed_count: i32,
        pub valid_until: Option<DateTime<Utc>>,
        pub active: bool,
    }

    pub struct CreateGiftCardRequest {
        pub currency: Currency,
        pub amount_minor: i64,
    }

    pub struct GiftCardCreated {
        pub id: Uuid,
        /// Shown once; only a hash is stored.
        pub code: String,
        pub currency: Currency,
        pub amount_minor: i64,
    }

    pub struct CreateCreatorCodeRequest {
        pub code: String,
        pub creator_user_id: Uuid,
        /// Revenue share in basis points (500 = 5 %).
        pub revenue_share_bps: i32,
    }

    pub struct CreatorCode {
        pub id: Uuid,
        pub code: String,
        pub creator_user_id: Uuid,
        pub revenue_share_bps: i32,
        pub active: bool,
    }
}

// ---------------------------------------------------------------- servers & domains

wire! {
    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum DomainStatus {
        /// Records not verified yet.
        Pending,
        /// Ownership + routing verified: the relay routes this hostname.
        Verified,
        /// Last verification attempt failed (see per-record results).
        Failed,
        /// Was verified, records disappeared on a periodic re-check: routing stopped.
        Revoked,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum RecordCheckStatus {
        Unchecked,
        Ok,
        Missing,
        /// Record exists with a different value (see `found`).
        Mismatch,
        /// DNS lookup error (timeout, SERVFAIL...).
        Error,
    }

    /// Result of the last DNS check of one expected record.
    pub struct RecordCheck {
        pub status: RecordCheckStatus,
        /// Values actually found in DNS (truncated).
        pub found: Vec<String>,
        pub checked_at: Option<DateTime<Utc>>,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "UPPERCASE")]
    pub enum DnsRecordKind {
        Txt,
        Cname,
        Srv,
    }

    /// A DNS record to create at the user's registrar (any registrar: by hand, or by the launcher
    /// through the user's own registrar API keys). `name` is relative to the zone (`@` = apex).
    pub struct DnsRecord {
        pub kind: DnsRecordKind,
        /// Fully qualified record name.
        pub fqdn: String,
        /// Record name relative to the zone (what registrar APIs expect as the subdomain).
        pub name: String,
        /// TXT content, CNAME target, or SRV `priority weight port target`.
        pub value: String,
        pub ttl: u32,
        /// `verification`, `routing`, `minecraft_srv`.
        pub purpose: String,
        /// Required for verification. For routing, either the CNAME or the SRV record suffices
        /// (apex domains cannot hold a CNAME: use SRV).
        pub required: bool,
        pub check: RecordCheck,
    }

    pub struct CustomDomain {
        pub id: Uuid,
        /// Hostname players type (e.g. `play.example.com` or `example.com`).
        pub hostname: String,
        /// Registrable zone the records live in (e.g. `example.com`).
        pub zone: String,
        pub status: DomainStatus,
        /// True when the relay is authorised to route this hostname.
        pub routing_active: bool,
        pub verified_at: Option<DateTime<Utc>>,
        pub last_checked_at: Option<DateTime<Utc>>,
        /// Next periodic re-verification (verified domains).
        pub next_check_at: Option<DateTime<Utc>>,
        pub last_error: Option<String>,
        /// Registrar-agnostic record set to create, with per-record live check results.
        pub dns_records: Vec<DnsRecord>,
    }

    pub struct RelayServer {
        pub id: Uuid,
        pub name: String,
        /// Unique route name `<route-id>.<relay-host>`, always routed: the CNAME/SRV target for
        /// custom domains (ADR 0013), also usable directly by players.
        pub route_hostname: String,
        /// `<label>.<relay-domain>` when the optional free subdomain feature is enabled.
        pub subdomain: Option<String>,
        /// Hostnames the relay is authorised to route (verified domains + subdomain).
        pub hostnames: Vec<String>,
        pub domains: Vec<CustomDomain>,
        pub created_at: DateTime<Utc>,
    }

    pub struct CreateServerRequest {
        /// Display name, 3-32 chars.
        pub name: String,
        /// Request a free `<label>.<relay-domain>` (only when the feature is enabled).
        #[serde(default)]
        pub subdomain_label: Option<String>,
    }

    pub struct NameAvailability {
        pub label: String,
        pub available: bool,
        pub reason: Option<String>,
        pub feature_enabled: bool,
    }

    pub struct AddDomainRequest {
        pub hostname: String,
        /// Registrable zone; defaults to the last two labels of `hostname`.
        #[serde(default)]
        pub zone: Option<String>,
    }

    pub struct RelayToken {
        /// Tunnel token for the relay handshake (HS256 JWT, relay ADR 0101, docs/API.md).
        pub token: String,
        pub expires_at: DateTime<Utc>,
        /// Relay tunnel endpoint `host:port` (TLS).
        pub relay_addr: String,
        /// Permanent hostnames (verified custom domains, optional free subdomain).
        pub hostnames: Vec<String>,
        /// Temporary hostnames with their lease expiry.
        pub temp_hostnames: Vec<TempHostname>,
    }

    pub struct TempHostname {
        /// `<adjective>-<noun>-<NN>.<temp-domain>`, e.g. `brave-otter-42.tmp.relay.example.invalid`.
        pub hostname: String,
        pub expires_at: DateTime<Utc>,
    }

    /// A temporary relay hostname lease for a server running on the user's machine, plus a
    /// relay token that already carries it.
    pub struct TempHostnameLease {
        pub hostname: String,
        pub expires_at: DateTime<Utc>,
        /// Renew before this instant to keep the same hostname.
        pub renew_before: DateTime<Utc>,
        pub relay_token: RelayToken,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum HostingAvailability {
        ComingSoon,
        Available,
    }

    pub struct HostingPlan {
        pub id: String,
        pub name: String,
        pub memory_mb: i32,
        pub vcpus: f32,
        pub storage_gb: i32,
        pub max_players: i32,
        /// Indicative monthly prices (not purchasable yet).
        pub monthly_prices: Vec<PriceInput>,
    }

    pub struct HostingRegion {
        pub id: String,
        pub name: String,
        pub country: String,
    }

    /// Rustic Cloud hosting (coming soon): catalog preview, no provisioning.
    pub struct HostingCatalog {
        pub availability: HostingAvailability,
        pub plans: Vec<HostingPlan>,
        pub regions: Vec<HostingRegion>,
    }

    /// Also accepts the hosting crate's `{"region", "expectedPlayers"}` shape.
    pub struct WaitlistRequest {
        #[serde(default)]
        pub plan_id: Option<String>,
        #[serde(default, alias = "region")]
        pub region_id: Option<String>,
        #[serde(default, alias = "expectedPlayers")]
        pub expected_players: Option<i32>,
    }

    /// `GET /v1/hosting/plans` (shape of `rusticmc_hosting::cloud::CloudPlan`, camelCase).
    #[serde(rename_all = "camelCase")]
    pub struct HostingCloudPlan {
        pub id: String,
        pub name: String,
        pub ram_mb: i32,
        pub vcpus: f32,
        pub disk_gb: i32,
        pub max_players: i32,
        /// Monthly price, VAT included, minor units (indicative: not purchasable yet).
        pub price_cents: i64,
        pub currency: String,
    }

    /// `GET /v1/hosting/regions` (shape of `rusticmc_hosting::cloud::CloudRegion`).
    #[serde(rename_all = "camelCase")]
    pub struct HostingCloudRegion {
        pub id: String,
        pub name: String,
        pub country: String,
        pub available: bool,
    }

    /// An invite code for a server (ADR 0105): friends type it in the launcher, the cloud
    /// resolves it to the server's route hostname.
    pub struct InviteCode {
        pub code: String,
        pub server_id: Uuid,
        pub hostname: String,
        pub expires_at: DateTime<Utc>,
    }

    pub struct InviteCodeResolution {
        pub code: String,
        pub server_name: String,
        /// `host` to join (the relay route hostname; port 25565 unless `port` is set).
        pub hostname: String,
        pub port: i32,
        pub expires_at: DateTime<Utc>,
    }

    pub struct WaitlistStatus {
        pub joined: bool,
        pub joined_at: Option<DateTime<Utc>>,
        pub plan_id: Option<String>,
        pub region_id: Option<String>,
    }
}

// ---------------------------------------------------------------- GDPR & admin

wire! {
    pub struct DeletionStatus {
        pub scheduled_for: Option<DateTime<Utc>>,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum ReportStatus {
        Open,
        Actioned,
        Dismissed,
    }

    pub struct Report {
        pub id: Uuid,
        pub reporter_id: Option<Uuid>,
        pub target: PublicUser,
        pub message: Option<Message>,
        pub reason: ReportReason,
        pub details: Option<String>,
        pub status: ReportStatus,
        pub created_at: DateTime<Utc>,
        pub resolution_note: Option<String>,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum ModerationAction {
        Dismiss,
        ChatBan,
        Ban,
    }

    pub struct ResolveReportRequest {
        pub action: ModerationAction,
        #[serde(default)]
        pub note: Option<String>,
        /// Ban duration; `null` = permanent.
        #[serde(default)]
        pub duration_hours: Option<i64>,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum BanScope {
        Platform,
        Chat,
    }

    pub struct Ban {
        pub id: Uuid,
        pub user_id: Uuid,
        pub scope: BanScope,
        pub reason: String,
        pub created_at: DateTime<Utc>,
        pub expires_at: Option<DateTime<Utc>>,
        pub revoked_at: Option<DateTime<Utc>>,
    }

    pub struct CreateBanRequest {
        pub user_id: Uuid,
        pub scope: BanScope,
        pub reason: String,
        #[serde(default)]
        pub duration_hours: Option<i64>,
    }

    pub struct AuditEntry {
        pub id: i64,
        pub actor_id: Option<Uuid>,
        pub action: String,
        pub target_type: String,
        pub target_id: Option<String>,
        #[cfg_attr(feature = "openapi", schema(value_type = Object))]
        pub data: serde_json::Value,
        pub created_at: DateTime<Utc>,
    }
}

// ---------------------------------------------------------------- content registry
// JSON shapes of `rusticmc_content::model` (ProjectSummary, ContentVersion...), so the launcher's
// Rustic content source reads these answers as is.

wire! {
    #[derive(Copy, Eq, Hash)]
    pub enum ContentKind {
        #[serde(rename = "mod")]
        Mod,
        #[serde(rename = "native_mod")]
        NativeMod,
        #[serde(rename = "modpack")]
        Modpack,
        #[serde(rename = "resourcepack")]
        ResourcePack,
        #[serde(rename = "shader")]
        Shader,
        #[serde(rename = "shaderpack")]
        ShaderPack,
        #[serde(rename = "datapack")]
        DataPack,
        /// A Rustic Server plugin (`.rplugin`).
        #[serde(rename = "plugin")]
        Plugin,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum FairPlayStatus {
        Clean,
        /// Never promoted; installable with a notice.
        Flagged,
        /// Never returned by search, never installable through Rustic.
        Blocked,
    }

    /// Same JSON as `rusticmc_content::model::FairPlay` (`{"status": "clean"}`).
    pub struct ContentFairPlay {
        pub status: FairPlayStatus,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub reason: Option<String>,
    }

    pub struct ContentProject {
        /// Always `rustic`.
        pub source: String,
        pub id: String,
        pub slug: String,
        pub kind: ContentKind,
        pub name: String,
        pub summary: String,
        pub authors: Vec<String>,
        pub icon_url: Option<String>,
        pub downloads: i64,
        pub categories: Vec<String>,
        pub tags: Vec<String>,
        pub license: Option<String>,
        pub loaders: Vec<String>,
        pub game_versions: Vec<String>,
        pub fair_play: ContentFairPlay,
    }

    pub struct ContentSearchResponse {
        pub hits: Vec<ContentProject>,
    }

    pub struct ContentHashes {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub sha1: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub sha256: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub sha512: Option<String>,
    }

    pub struct ContentFile {
        pub url: String,
        pub filename: String,
        pub size: Option<i64>,
        pub hashes: ContentHashes,
        pub primary: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub signature: Option<String>,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum ContentDependencyKind {
        Required,
        Optional,
        Incompatible,
        Embedded,
    }

    pub struct ContentDependency {
        #[serde(default)]
        pub project_id: Option<String>,
        #[serde(default)]
        pub version_id: Option<String>,
        /// Semver requirement (Rustic plugins).
        #[serde(default)]
        pub version_req: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub sha256: Option<String>,
        pub kind: ContentDependencyKind,
    }

    pub struct ContentVersion {
        /// Always `rustic`.
        pub source: String,
        pub project_id: String,
        /// `<project>@<version>`.
        pub id: String,
        pub name: String,
        pub version: String,
        pub kind: ContentKind,
        pub game_versions: Vec<String>,
        pub loaders: Vec<String>,
        pub files: Vec<ContentFile>,
        pub dependencies: Vec<ContentDependency>,
        /// RFC 3339.
        pub published: String,
    }

    pub struct FairPlayDenyEntry {
        /// Project id or slug (any source: `modrinth:<id>`, `horizon:<id>`, a bare slug...).
        pub key: String,
        pub reason: String,
    }

    /// The authoritative fair-play deny list (X-ray, auto-clickers, hacks...).
    pub struct FairPlayDenyList {
        pub entries: Vec<FairPlayDenyEntry>,
        pub updated_at: Option<DateTime<Utc>>,
    }

    pub struct CreateContentProjectRequest {
        pub slug: String,
        pub kind: ContentKind,
        pub name: String,
        pub summary: String,
        #[serde(default)]
        pub authors: Vec<String>,
        #[serde(default)]
        pub icon_url: Option<String>,
        #[serde(default)]
        pub categories: Vec<String>,
        #[serde(default)]
        pub tags: Vec<String>,
        #[serde(default)]
        pub license: Option<String>,
        #[serde(default)]
        pub loaders: Vec<String>,
        #[serde(default)]
        pub game_versions: Vec<String>,
    }

    pub struct CreateContentVersionRequest {
        pub version: String,
        pub name: String,
        #[serde(default)]
        pub game_versions: Vec<String>,
        #[serde(default)]
        pub loaders: Vec<String>,
        pub files: Vec<ContentFile>,
        #[serde(default)]
        pub dependencies: Vec<ContentDependency>,
    }

    pub struct DenyEntryRequest {
        pub reason: String,
    }
}

// ---------------------------------------------------------------- news & featured servers (ADR 0140)
// Published by staff from the website back-office; read by the launcher's Launchpad and the site.

wire! {
    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum NewsSource {
        Rustic,
        Horizon,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum NewsKind {
        PatchNotes,
        Event,
        Blog,
    }

    /// A published news item (public).
    pub struct NewsItem {
        pub id: Uuid,
        pub slug: String,
        pub source: NewsSource,
        pub kind: NewsKind,
        pub title: String,
        pub summary: String,
        /// Markdown body (rendered as text by the clients; no raw HTML).
        pub body: String,
        /// Where "Read more" goes: the external link when set, else the website's news page.
        pub url: String,
        /// Version the item is about (patch notes), if any.
        pub version: Option<String>,
        /// BCP 47 language of the item (`en`, `fr`).
        pub locale: String,
        pub published_at: DateTime<Utc>,
    }

    /// A news item as staff see it (drafts included).
    pub struct AdminNewsItem {
        pub id: Uuid,
        pub slug: String,
        pub source: NewsSource,
        pub kind: NewsKind,
        pub title: String,
        pub summary: String,
        pub body: String,
        pub external_url: Option<String>,
        pub version: Option<String>,
        pub locale: String,
        /// Null while a draft.
        pub published_at: Option<DateTime<Utc>>,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
    }

    pub struct NewsUpsertRequest {
        /// 2-64 chars `[a-z0-9-]`, unique.
        pub slug: String,
        pub source: NewsSource,
        pub kind: NewsKind,
        pub title: String,
        pub summary: String,
        #[serde(default)]
        pub body: String,
        #[serde(default)]
        pub external_url: Option<String>,
        #[serde(default)]
        pub version: Option<String>,
        /// Default `en`.
        #[serde(default)]
        pub locale: Option<String>,
        /// Publish now (true) or keep as a draft (false).
        pub published: bool,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum FeaturedEngine {
        Horizon,
        Rustic,
    }

    /// A server staff feature on the Launchpad and the Servers page.
    pub struct FeaturedServer {
        pub id: Uuid,
        pub name: String,
        /// `host` or `host:port` players type in the game.
        pub address: String,
        pub description: String,
        pub tags: Vec<String>,
        pub engine: FeaturedEngine,
        /// BCP 47 language of the community.
        pub language: String,
        /// Staff checked the owner and the rules.
        pub verified: bool,
        /// Lower first.
        pub sort_order: i32,
        /// Not measured by the cloud yet: clients ping the server themselves.
        pub players_online: Option<i32>,
        pub players_max: Option<i32>,
    }

    /// A featured server as staff see it.
    pub struct AdminFeaturedServer {
        pub server: FeaturedServer,
        pub active: bool,
        pub created_at: DateTime<Utc>,
    }

    pub struct FeaturedServerUpsertRequest {
        pub name: String,
        pub address: String,
        #[serde(default)]
        pub description: String,
        #[serde(default)]
        pub tags: Vec<String>,
        pub engine: FeaturedEngine,
        /// Default `en`.
        #[serde(default)]
        pub language: Option<String>,
        #[serde(default)]
        pub verified: bool,
        #[serde(default)]
        pub sort_order: i32,
        /// Shown to players (true) or hidden (false).
        pub active: bool,
    }
}

// ---------------------------------------------------------------- in-game companion (ADR 0071)
// Shapes of `client-mod/rustic-companion/core/src/protocol.rs`. UUIDs are 32-hex without dashes.

wire! {
    pub struct WornRequest {
        /// Up to 64 Minecraft UUIDs (hex, dashes accepted).
        pub players: Vec<String>,
    }

    /// One worn cosmetic: where its bundle is on the CDN.
    pub struct WornItem {
        pub id: String,
        pub version: String,
        /// SHA-256 of the bundle.
        pub hash: String,
    }

    pub struct WornPlaying {
        pub item: WornItem,
        /// Milliseconds since the emote started.
        pub ms: i64,
    }

    pub struct WornAnswer {
        /// Seconds this answer may be cached (60-3600).
        pub ttl: i64,
        /// By UUID hex; players wearing nothing are left out.
        pub players: std::collections::BTreeMap<String, Vec<WornItem>>,
        /// Emotes playing now, by UUID hex.
        pub playing: std::collections::BTreeMap<String, WornPlaying>,
    }

    pub struct CompanionFriend {
        /// Minecraft UUID hex of the friend's linked profile (Rustic user id hex if none).
        pub uuid: String,
        pub name: String,
        pub online: bool,
        pub activity: Option<String>,
    }

    pub struct CompanionInvite {
        pub id: String,
        pub from: String,
        pub server: String,
    }

    pub struct CompanionMessage {
        pub id: String,
        pub from: String,
        pub text: String,
    }

    pub struct SocialSnapshot {
        pub friends: Vec<CompanionFriend>,
        pub invites: Vec<CompanionInvite>,
        /// Unread direct messages (most recent 20).
        pub messages: Vec<CompanionMessage>,
        /// Emotes the account owns (the wheel shows at most 8).
        pub emotes: Vec<WornItem>,
    }

    pub struct CompanionInviteRequest {
        /// Friend's Minecraft UUID hex (or Rustic user id).
        pub to: String,
        /// `host[:port]` of the server the inviter is on.
        pub address: String,
    }

    pub struct EmotePlayingRequest {
        /// Emote id (`asset_ref` of an owned emote) or `null` to stop.
        pub emote: Option<String>,
    }
}

impl Currency {
    pub const ALL: [Currency; 4] = [Currency::Eur, Currency::Usd, Currency::Gbp, Currency::Chf];

    pub fn code(self) -> &'static str {
        match self {
            Currency::Eur => "EUR",
            Currency::Usd => "USD",
            Currency::Gbp => "GBP",
            Currency::Chf => "CHF",
        }
    }

    pub fn parse(code: &str) -> Option<Currency> {
        Self::ALL
            .into_iter()
            .find(|c| c.code().eq_ignore_ascii_case(code))
    }
}

impl CosmeticSlot {
    pub const ALL: [CosmeticSlot; 9] = [
        CosmeticSlot::Head,
        CosmeticSlot::Back,
        CosmeticSlot::Top,
        CosmeticSlot::ShoulderCompanion,
        CosmeticSlot::Companion,
        CosmeticSlot::Trail,
        CosmeticSlot::Aura,
        CosmeticSlot::Emote,
        CosmeticSlot::Nametag,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            CosmeticSlot::Head => "head",
            CosmeticSlot::Back => "back",
            CosmeticSlot::Top => "top",
            CosmeticSlot::ShoulderCompanion => "shoulder_companion",
            CosmeticSlot::Companion => "companion",
            CosmeticSlot::Trail => "trail",
            CosmeticSlot::Aura => "aura",
            CosmeticSlot::Emote => "emote",
            CosmeticSlot::Nametag => "nametag",
        }
    }

    pub fn parse(s: &str) -> Option<CosmeticSlot> {
        Self::ALL.into_iter().find(|c| c.as_str() == s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn currency_round_trip() {
        for c in Currency::ALL {
            assert_eq!(Currency::parse(c.code()), Some(c));
            let json = serde_json::to_string(&c).unwrap();
            assert_eq!(json, format!("\"{}\"", c.code()));
        }
    }

    #[test]
    fn slot_round_trip() {
        for s in CosmeticSlot::ALL {
            let json = serde_json::to_string(&s).unwrap();
            assert_eq!(json, format!("\"{}\"", s.as_str()));
            assert_eq!(CosmeticSlot::parse(s.as_str()), Some(s));
        }
    }

    #[test]
    fn login_request_defaults_to_launcher() {
        let r: LoginRequest = serde_json::from_str(r#"{"email":"a@b.c","password":"x"}"#).unwrap();
        assert_eq!(r.client, ClientKind::Launcher);
        assert!(r.device_name.is_none());
    }
}
