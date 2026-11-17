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
        /// The official Rustic account (ADR 0166): shown with an "Official" mark. It publishes
        /// Rustic's own content and never signs in.
        #[serde(default)]
        pub official: bool,
        /// The account's avatar (a 64x64 PNG on the CDN, ADR 0168): a chosen default head, the
        /// head of an uploaded skin, or the default head assigned from the account id. `None`
        /// only from services that cannot build CDN URLs.
        #[serde(default)]
        pub avatar_url: Option<String>,
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
        /// The account has an email address (needed to sign in with a password).
        #[serde(default)]
        pub has_email: bool,
        /// That address is verified (a password can only be set on a verified address).
        #[serde(default)]
        pub email_verified: bool,
    }

    /// Removes the password (ADR 0168): allowed only while another sign-in method (Microsoft)
    /// stays linked.
    pub struct RemovePasswordRequest {
        pub current_password: String,
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
        /// May talk in party voice (party members are friends; ADR 0176): the chat rules, plus a
        /// linked guardian's `party_voice` switch.
        #[serde(default)]
        pub can_party_voice: bool,
        /// May join the proximity voice of a Rustic Server, with strangers (ADR 0176): adults,
        /// and minors whose linked guardian turned `server_voice` on.
        #[serde(default)]
        pub can_server_voice: bool,
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
        /// The account's avatar (ADR 0168), as others see it.
        #[serde(default)]
        pub avatar_url: Option<String>,
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
        /// Party voice with friends (ADR 0176; on by default, off when chat is disabled).
        #[serde(default = "crate::types::yes")]
        pub party_voice: bool,
        /// The proximity voice of Rustic Servers, with strangers (ADR 0176; off by default).
        #[serde(default)]
        pub server_voice: bool,
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
        /// The party's voice settings (ADR 0176).
        #[serde(default)]
        pub voice: PartyVoice,
        /// Invites sent and still pending (who was asked and has not answered).
        #[serde(default)]
        pub pending_invites: Vec<PublicUser>,
    }

    #[derive(Default)]
    /// How party members hear each other (ADR 0176).
    pub struct PartyVoice {
        /// `true`: members on the same server hear each other by distance in game; `false`:
        /// everyone at full volume, wherever they are.
        pub proximity: bool,
        /// Blocks beyond which a member is silent in proximity mode (8-128, default 48).
        pub range: i32,
    }

    /// `PATCH /v1/parties/{id}` (leader only): a field left out is kept.
    pub struct PartySettingsRequest {
        #[serde(default)]
        pub proximity: Option<bool>,
        #[serde(default)]
        pub range: Option<i32>,
    }

    /// A message of the party's text chat.
    pub struct PartyMessage {
        pub id: Uuid,
        pub party_id: Uuid,
        pub sender_id: Uuid,
        pub body: String,
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
        /// Content that infringes someone's rights (copyright, trademark).
        Copyright,
        /// Content that harms players' computers or accounts.
        Malware,
        Other,
    }

    pub struct ReportRequest {
        pub target_user_id: Uuid,
        #[serde(default)]
        pub message_id: Option<Uuid>,
        pub reason: ReportReason,
        #[serde(default)]
        pub details: Option<String>,
        /// Where it happened: a voice room or the party chat (ADR 0176).
        #[serde(default)]
        pub context: Option<ReportContext>,
        /// A party chat message the target sent (`context` = `party_chat`).
        #[serde(default)]
        pub party_message_id: Option<Uuid>,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum ReportContext {
        PartyVoice,
        ServerVoice,
        PartyChat,
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
        /// Not usable before (ADR 0171).
        #[serde(default)]
        pub starts_at: Option<DateTime<Utc>>,
        /// Uses per account (default 1).
        #[serde(default)]
        pub per_user_limit: Option<i32>,
        /// Minimum basket in `currency` (needs a currency).
        #[serde(default)]
        pub min_subtotal_minor: Option<i64>,
        #[serde(default)]
        pub note: Option<String>,
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
        /// A marketplace project report (the target is its owner).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub content_project: Option<ReportedContent>,
        /// A server listing report (the target is its owner). ADR 0158.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub server_listing: Option<ReportedServer>,
        /// A voice room or the party chat (ADR 0176).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub context: Option<ReportContext>,
        /// The reported party chat message.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub party_message: Option<PartyMessage>,
    }

    pub struct ReportedServer {
        pub id: Uuid,
        pub slug: String,
        pub name: String,
        pub address: String,
    }

    pub struct ReportedContent {
        pub id: String,
        pub slug: String,
        pub name: String,
        pub kind: ContentKind,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum ModerationAction {
        Dismiss,
        ChatBan,
        Ban,
        /// Content and server listing reports: takes the reported project or listing down (the
        /// note is the reason).
        TakeDown,
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
        /// Caller address when recorded (deleted with the actor's account).
        #[serde(default)]
        pub ip: Option<String>,
        /// Caller user agent when recorded (back-office actions since ADR 0169).
        #[serde(default)]
        pub user_agent: Option<String>,
        /// `info`, `notice`, `warning` or `critical`, from the action (ADR 0169).
        #[serde(default = "default_severity")]
        pub severity: String,
        /// The actor's account when it still exists.
        #[serde(default)]
        pub actor: Option<AdminUserRef>,
    }
}

fn default_severity() -> String {
    "info".into()
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
        // Marketplace fields (ADR 0155). Optional on the wire: older readers ignore them, and
        // `rusticmc_content::model::ProjectSummary` reads the fields above only.
        /// Markdown page; only in `GET /v1/content/projects/{id}` (not in search hits).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        /// Screenshots, in order (re-encoded on upload, served from Rustic's CDN).
        #[serde(default)]
        pub gallery: Vec<ContentImage>,
        #[serde(default)]
        pub links: ContentLinks,
        /// The Rustic account that publishes it (`None` for curated entries of a deleted admin).
        #[serde(default)]
        pub owner: Option<PublicUser>,
        #[serde(default)]
        pub visibility: ContentVisibility,
        /// An admin confirmed the project matches its description.
        #[serde(default)]
        pub verified: bool,
        #[serde(default)]
        pub price: ContentPrice,
        #[serde(default)]
        pub created_at: Option<DateTime<Utc>>,
        #[serde(default)]
        pub updated_at: Option<DateTime<Utc>>,
        /// Newest version visible to the caller.
        #[serde(default)]
        pub latest_version: Option<String>,
        /// Size in bytes of the newest version's primary file.
        #[serde(default)]
        pub size: Option<i64>,
        /// `pending` while a native mod waits for an admin (it is then private whatever its
        /// visibility says).
        #[serde(default)]
        pub review: ContentReviewStatus,
        /// Hosted by Rustic's marketplace (uploaded by its creator) rather than curated.
        #[serde(default)]
        pub marketplace: bool,
        /// Owner and staff only: why it is under review, rejected or taken down.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub moderation: Option<ContentModeration>,
    }

    pub struct ContentSearchResponse {
        pub hits: Vec<ContentProject>,
        /// More results after `offset + hits.len()`.
        #[serde(default)]
        pub has_more: bool,
        /// Matching projects in all.
        #[serde(default)]
        pub total: i64,
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
        /// What the file's manifest says, in the JSON of `rusticmc_content::model::HorizonFacts`
        /// (the launcher's compatibility verdicts read it). Marketplace versions only.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub horizon: Option<ContentHorizonFacts>,
        #[serde(default)]
        pub changelog: String,
        #[serde(default)]
        pub channel: ContentChannel,
        /// Yanked versions are listed to their owner and staff only.
        #[serde(default)]
        pub yanked: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub yanked_reason: Option<String>,
        #[serde(default)]
        pub downloads: i64,
        #[serde(default)]
        pub review: ContentReviewStatus,
        /// Owner and staff only: the reviewer's note.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub review_note: Option<String>,
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

// ---------------------------------------------------------------- marketplace (ADR 0155)
// Creators publish from the website; the launcher lists and installs public content through the
// content registry types above. Uploads are checked server side before anything is published.

wire! {
    #[derive(Copy, Eq, Hash, Default)]
    #[serde(rename_all = "snake_case")]
    pub enum ContentVisibility {
        /// Listed, searchable and installable.
        #[default]
        Public,
        /// Not listed; reachable and installable through its link.
        Unlisted,
        /// Its owner (and admins) only.
        Private,
    }

    #[derive(Copy, Eq, Hash, Default)]
    #[serde(rename_all = "snake_case")]
    pub enum ContentReviewStatus {
        /// Data content: published after the automatic checks.
        #[default]
        NotRequired,
        /// A native mod waiting for an admin: private until approved.
        Pending,
        Approved,
        Rejected,
    }

    #[derive(Copy, Eq, Hash, Default)]
    #[serde(rename_all = "snake_case")]
    pub enum ContentChannel {
        #[default]
        Release,
        Beta,
        Alpha,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum ContentUploadPurpose {
        /// The content file of a version (`.hmod`, `.hnmod`, `.hpack`, `.rusticpack`, `.hshader`,
        /// a resource or data pack `.zip`).
        File,
        /// The project's icon (square PNG, JPEG or WebP; re-encoded as PNG).
        Icon,
        /// A screenshot (PNG, JPEG or WebP; re-encoded).
        Gallery,
        /// A server listing's banner (PNG, JPEG or WebP, 960x300 to 3840x2160; re-encoded, at
        /// most 1920 px wide). ADR 0158.
        Banner,
    }

    #[derive(Copy, Eq, Hash, Default)]
    #[serde(rename_all = "snake_case")]
    pub enum LibrarySource {
        /// A free project the user added.
        #[default]
        Free,
        /// Bought (creator sales: not available yet).
        Purchase,
        /// Given by staff.
        Grant,
    }

    pub struct ContentImage {
        pub url: String,
        pub width: i32,
        pub height: i32,
    }

    /// Links shown on the project page (`https://` only).
    #[derive(Default)]
    pub struct ContentLinks {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub source: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub issues: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub wiki: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub discord: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub website: Option<String>,
    }

    /// `amount_cents` 0 = free. Paid creator content is shown with its price; buying it is not
    /// available yet (ADR 0155).
    pub struct ContentPrice {
        pub amount_cents: i64,
        pub currency: Currency,
    }

    /// Owner and staff view of a project's moderation state.
    pub struct ContentModeration {
        pub taken_down: bool,
        pub takedown_reason: Option<String>,
        pub taken_down_at: Option<DateTime<Utc>>,
        pub verified_at: Option<DateTime<Utc>>,
        /// Versions waiting for an admin (native mods).
        pub pending_versions: i64,
        /// The newest reviewer note on a rejected version.
        pub review_note: Option<String>,
    }

    /// The JSON of `rusticmc_content::model::HorizonFacts` (every key present).
    pub struct ContentHorizonFacts {
        /// A content mod's API line (`0.2`), a native mod's `native-1/<horizon>/<mixins>`, a
        /// modpack's format, a shader pack's shader API, a resource/data pack's pack format.
        pub api: String,
        /// A native mod's hook points version.
        pub hooks: Option<u32>,
        pub side: Option<String>,
        pub capabilities: Vec<String>,
        pub broken_on: Vec<String>,
        pub key: Option<String>,
    }

    /// What a checked file says about itself (its manifest).
    pub struct ContentFileFacts {
        pub kind: ContentKind,
        /// The manifest's id (content mods, native mods, `.hpack`, `.hshader`).
        pub id: Option<String>,
        pub name: Option<String>,
        pub version: Option<String>,
        /// See [`ContentHorizonFacts::api`].
        pub api: String,
        /// The game version it is for, when the manifest names one.
        pub game: Option<String>,
        pub hooks: Option<u32>,
        pub side: Option<String>,
        pub capabilities: Vec<String>,
        /// Files in the archive.
        pub entries: i64,
        /// Unpacked size in bytes.
        pub unpacked_size: i64,
    }

    /// A staged upload, checked and kept until a publish request uses it (or it expires).
    pub struct ContentUpload {
        pub id: Uuid,
        pub purpose: ContentUploadPurpose,
        pub filename: String,
        pub size: i64,
        pub sha256: String,
        pub sha512: String,
        /// Content files.
        pub facts: Option<ContentFileFacts>,
        /// Images.
        pub width: Option<i32>,
        pub height: Option<i32>,
        pub expires_at: DateTime<Utc>,
    }

    /// The first (or a new) version of a project.
    pub struct NewContentVersion {
        /// A staged `file` upload.
        pub file_upload: Uuid,
        /// Must equal the version in the file's manifest when it has one.
        pub version: String,
        /// Display name; the version when empty.
        #[serde(default)]
        pub name: Option<String>,
        /// Markdown.
        #[serde(default)]
        pub changelog: String,
        #[serde(default)]
        pub channel: ContentChannel,
        /// Game versions it is for (`26.3`); the manifest's when empty.
        #[serde(default)]
        pub game_versions: Vec<String>,
    }

    /// Publishes a new project with its first version (multi-step form of the website).
    pub struct PublishContentRequest {
        pub kind: ContentKind,
        /// `[a-z0-9_-]{2,64}`; the project id. For a content mod, native mod, `.hpack` or
        /// `.hshader`, it must be the manifest's id (`-` read as `_`).
        pub slug: String,
        pub name: String,
        /// One line, at most 160 characters.
        pub summary: String,
        /// Markdown, at most 10,000 characters.
        #[serde(default)]
        pub description: String,
        /// 1 to 3 of the kind's categories (`GET /v1/content/catalog`).
        pub categories: Vec<String>,
        /// At most 10, `[a-z0-9][a-z0-9-]{0,23}`.
        #[serde(default)]
        pub tags: Vec<String>,
        /// An SPDX expression or `All-Rights-Reserved`.
        pub license: String,
        #[serde(default)]
        pub links: ContentLinks,
        pub visibility: ContentVisibility,
        /// 0 = free.
        #[serde(default)]
        pub price_cents: i64,
        #[serde(default)]
        pub currency: Option<Currency>,
        /// A staged `icon` upload.
        #[serde(default)]
        pub icon_upload: Option<Uuid>,
        /// Staged `gallery` uploads, in order (at most 8).
        #[serde(default)]
        pub gallery_uploads: Vec<Uuid>,
        pub version: NewContentVersion,
        /// The publishing agreement: the creator owns the rights, the content follows the rules
        /// and the fair-play policy. Must be `true`.
        pub accept_terms: bool,
    }

    pub struct PublishContentResponse {
        pub project: ContentProject,
        pub version: ContentVersion,
    }

    /// The official Rustic publisher account and what it published (ADR 0166, staff only).
    pub struct OfficialPublisher {
        /// The account (`official: true`); it never signs in, staff publish as it.
        pub account: PublicUser,
        pub projects: Vec<OfficialProject>,
    }

    /// One project of the official account with every version (yanked and pending included).
    pub struct OfficialProject {
        pub project: ContentProject,
        pub versions: Vec<ContentVersion>,
    }

    /// One gallery image to keep: an existing one by `url`, or a staged upload.
    pub struct GalleryRef {
        #[serde(default)]
        pub url: Option<String>,
        #[serde(default)]
        pub upload_id: Option<Uuid>,
    }

    /// Every field optional: only the ones sent change.
    pub struct UpdateContentProjectRequest {
        #[serde(default)]
        pub name: Option<String>,
        #[serde(default)]
        pub summary: Option<String>,
        #[serde(default)]
        pub description: Option<String>,
        #[serde(default)]
        pub categories: Option<Vec<String>>,
        #[serde(default)]
        pub tags: Option<Vec<String>>,
        #[serde(default)]
        pub license: Option<String>,
        #[serde(default)]
        pub links: Option<ContentLinks>,
        #[serde(default)]
        pub visibility: Option<ContentVisibility>,
        #[serde(default)]
        pub price_cents: Option<i64>,
        #[serde(default)]
        pub currency: Option<Currency>,
        /// A staged `icon` upload.
        #[serde(default)]
        pub icon_upload: Option<Uuid>,
        /// The whole gallery, in order.
        #[serde(default)]
        pub gallery: Option<Vec<GalleryRef>>,
    }

    pub struct YankContentVersionRequest {
        /// `false` restores a yanked version.
        pub yanked: bool,
        #[serde(default)]
        pub reason: Option<String>,
    }

    pub struct ContentReportRequest {
        pub reason: ReportReason,
        #[serde(default)]
        pub details: Option<String>,
    }

    /// A short-lived link to a version's file (private files need one in a browser).
    pub struct ContentDownloadLink {
        pub url: String,
        pub expires_at: DateTime<Utc>,
    }

    pub struct LibraryEntry {
        pub project: ContentProject,
        pub source: LibrarySource,
        pub added_at: DateTime<Utc>,
    }

    pub struct ContentKindCategories {
        pub kind: ContentKind,
        pub categories: Vec<String>,
    }

    /// The kinds open to creators, their categories and the upload limits.
    pub struct ContentCatalogInfo {
        pub kinds: Vec<ContentKindCategories>,
        pub max_file_bytes: i64,
        pub max_image_bytes: i64,
        pub max_gallery: i64,
        /// False until creator sales open (Stripe Connect): paid projects are shown, not sold.
        pub creator_sales_enabled: bool,
    }

    // admin

    pub struct AdminContentList {
        pub items: Vec<ContentProject>,
        pub has_more: bool,
        pub total: i64,
    }

    pub struct ContentReviewItem {
        pub project: ContentProject,
        pub version: ContentVersion,
    }

    pub struct ContentReviewRequest {
        /// `true` approves the version (the project becomes visible as its owner chose), `false`
        /// rejects it with `note` as the reason.
        pub approve: bool,
        #[serde(default)]
        pub note: Option<String>,
    }

    pub struct ContentTakedownRequest {
        pub reason: String,
    }
}

impl Default for ContentPrice {
    fn default() -> Self {
        Self {
            amount_cents: 0,
            currency: Currency::Eur,
        }
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

    // ------------------------------------------------ the news page's updates feed (ADR 0177)

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum NewsActivityKind {
        /// Commits pushed to a repository's default branch.
        Push,
        /// A GitHub release.
        Release,
        /// A launcher version or Rustic edition build published on the CDN.
        CdnRelease,
    }

    /// One commit of a push. `url` is null for private repositories.
    pub struct NewsActivityCommit {
        /// Short sha (7 characters).
        pub sha: String,
        /// The first line of the message (secrets, tokens and emails redacted).
        pub message: String,
        /// GitHub login or name.
        pub author: String,
        pub url: Option<String>,
        pub at: Option<DateTime<Utc>>,
    }

    /// A repository of an entry: one, or every mirror the same commits were pushed to (ADR 0178).
    pub struct NewsActivityRepo {
        pub name: String,
        pub private: bool,
        /// That repository's compare page; null for a private repository.
        pub url: Option<String>,
    }

    /// An entry of the updates feed: a push, a GitHub release or a CDN release. Entries of private
    /// repositories never carry a URL (`url` and every commit's `url` are null). The pushes of one
    /// monorepo sync to several mirror repositories are one entry listing them all in `repos`.
    pub struct NewsActivity {
        pub id: i64,
        pub kind: NewsActivityKind,
        /// Repository name (`rustic`), or `launcher` / `edition` for CDN releases; for a grouped
        /// push, the first public repository of `repos` (or the first one).
        pub repo: String,
        /// Whether `repo` is private (for a grouped push: every repository is private).
        pub private: bool,
        /// Every repository of the entry, `repo` first (one for releases).
        #[serde(default)]
        pub repos: Vec<NewsActivityRepo>,
        pub branch: Option<String>,
        /// Compare page (push), release page (GitHub release) or the download page (CDN release).
        pub url: Option<String>,
        /// Tag or version (releases).
        pub version: Option<String>,
        /// Release name.
        pub title: Option<String>,
        pub prerelease: bool,
        pub forced: bool,
        /// Who pushed or published (GitHub login).
        pub actor: Option<String>,
        /// The push's commits, newest first: at most 5 in lists, every recorded one (≤ 50) on the
        /// entry itself.
        pub commits: Vec<NewsActivityCommit>,
        /// Commits in the push (can be more than `commits`).
        pub commit_count: i32,
        /// Release notes (Markdown), only on the entry itself (`GET /v1/news/activity/{id}`).
        pub body: Option<String>,
        pub created_at: DateTime<Utc>,
    }

    /// A page of the updates feed, newest first.
    pub struct NewsActivityPage {
        pub items: Vec<NewsActivity>,
        /// Pass as `before` for the next page; null on the last page.
        pub next: Option<String>,
        /// Repositories that have entries (for filters), by name.
        pub repos: Vec<String>,
    }

    /// An entry as staff see it in the back-office.
    pub struct AdminNewsActivity {
        pub entry: NewsActivity,
        pub hidden: bool,
    }

    pub struct AdminNewsActivityPage {
        pub items: Vec<AdminNewsActivity>,
        pub next: Option<String>,
    }

    pub struct NewsActivityPatch {
        /// Hide the entry from the public feed (true) or show it again (false).
        pub hidden: bool,
    }

    /// A person who committed to the organisation's repositories (ADR 0178).
    pub struct NewsContributor {
        /// GitHub login.
        pub login: String,
        /// Commits across every repository (GitHub's contributors statistics).
        pub commits: i64,
        /// The avatar, served by this API (`/v1/news/contributors/{login}/avatar?v=…`); null when
        /// there is none (the website draws an identicon).
        pub avatar_url: Option<String>,
        /// `https://github.com/{login}`.
        pub profile_url: String,
    }

    /// The contributors wall, most commits first.
    pub struct NewsContributorsPage {
        /// Contributors shown on the website (can be more than `items`).
        pub total: i64,
        pub items: Vec<NewsContributor>,
    }

    /// A contributor as staff see it in the back-office.
    pub struct AdminNewsContributor {
        pub contributor: NewsContributor,
        pub hidden: bool,
    }

    pub struct AdminNewsContributorsPage {
        pub items: Vec<AdminNewsContributor>,
    }

    pub struct NewsContributorPatch {
        /// Hide the contributor from the website (true) or show them again (false).
        pub hidden: bool,
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

// ---------------------------------------------------------------- server directory (ADR 0158)
// Players publish their servers from the website; the launcher's Servers page, the website's
// directory and (later) the in-game Discover screen read them. Staff "featured servers" are a flag
// on listings (`GET /v1/featured-servers` keeps its shape for older clients).

wire! {
    /// Last status the cloud read with the server list ping (public addresses only, every ~10 min).
    pub struct ServerListingStatus {
        /// Answered the last ping.
        pub online: bool,
        pub players_online: Option<i32>,
        pub players_max: Option<i32>,
        /// Plain text of the MOTD (formatting codes removed, at most 200 characters).
        pub motd: Option<String>,
        /// The version name the server announced.
        pub version: Option<String>,
        pub checked_at: DateTime<Utc>,
    }

    /// What the owner and staff see besides the public page.
    pub struct ServerListingModeration {
        pub hidden: bool,
        pub hidden_reason: Option<String>,
        pub hidden_at: Option<DateTime<Utc>>,
        pub verified_at: Option<DateTime<Utc>>,
        /// Position among staff picks (lower first); `null` when not featured.
        pub featured_rank: Option<i32>,
    }

    /// A server in the directory. Lists leave `description` out (`null`); the detail has it.
    pub struct ServerListing {
        pub id: Uuid,
        /// `[a-z0-9-]{3,48}`, unique: `/servers/<slug>` on the website, `rustic://server/<slug>`.
        pub slug: String,
        pub name: String,
        /// `host` or `host:port` players type in the game (lowercase).
        pub address: String,
        /// One line, 10-160 characters.
        pub summary: String,
        /// Markdown (the website's small subset; never raw HTML). Detail only.
        pub description: Option<String>,
        /// From `GET /v1/server-listings/catalog` (`survival`, `minigames`...), 1-6.
        pub tags: Vec<String>,
        /// Languages spoken (ISO 639-1, lowercase), 1-5.
        pub languages: Vec<String>,
        /// What the server runs: vanilla Horizon or Rustic Server (plugins).
        pub engine: FeaturedEngine,
        /// Game version players need (`26.3`), when the owner said.
        pub game_version: Option<String>,
        /// Players need the Rustic edition client (cosmetics, companion, native mods).
        pub requires_rustic_edition: bool,
        pub website: Option<String>,
        /// A Discord invite (`https://discord.gg/...`).
        pub discord: Option<String>,
        /// Square icon on Rustic's CDN.
        pub icon_url: Option<String>,
        /// Wide banner on Rustic's CDN.
        pub banner_url: Option<String>,
        pub visibility: ContentVisibility,
        /// Staff checked the owner and the rules.
        pub verified: bool,
        /// A staff pick (the Launchpad, the hero carousels).
        pub featured: bool,
        pub owner: Option<PublicUser>,
        pub status: Option<ServerListingStatus>,
        /// "Play" presses from the launcher over the last 7 days (anonymous, one per player and
        /// 6 hours): the trending signal with the player count.
        pub plays_7d: i64,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
        /// Owner and staff only.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub moderation: Option<ServerListingModeration>,
    }

    pub struct ServerListingPage {
        pub items: Vec<ServerListing>,
        /// More results after `offset + items.len()`.
        pub has_more: bool,
        /// Matching listings in all.
        pub total: i64,
    }

    /// The directory's vocabulary and limits (the forms and filters read it).
    pub struct ServerListingCatalog {
        pub tags: Vec<String>,
        /// Languages listings use most, most used first (filters).
        pub languages: Vec<String>,
        pub max_tags: i64,
        pub max_languages: i64,
        pub max_listings_per_user: i64,
        pub max_image_bytes: i64,
    }

    /// Create (`POST`) a listing. Published at once after the automatic checks.
    pub struct ServerListingRequest {
        pub slug: String,
        pub name: String,
        pub address: String,
        pub summary: String,
        #[serde(default)]
        pub description: String,
        pub tags: Vec<String>,
        pub languages: Vec<String>,
        pub engine: FeaturedEngine,
        #[serde(default)]
        pub game_version: Option<String>,
        #[serde(default)]
        pub requires_rustic_edition: bool,
        #[serde(default)]
        pub website: Option<String>,
        #[serde(default)]
        pub discord: Option<String>,
        #[serde(default)]
        pub visibility: ContentVisibility,
        /// A staged `icon` upload (`POST /v1/content/uploads?purpose=icon`).
        #[serde(default)]
        pub icon_upload: Option<Uuid>,
        /// A staged `banner` upload.
        #[serde(default)]
        pub banner_upload: Option<Uuid>,
        /// The owner accepts the server listing rules (no impersonation, honest description).
        pub accept_rules: bool,
    }

    /// Edit (`PATCH`): absent fields stay as they are.
    pub struct UpdateServerListingRequest {
        #[serde(default)]
        pub name: Option<String>,
        #[serde(default)]
        pub address: Option<String>,
        #[serde(default)]
        pub summary: Option<String>,
        #[serde(default)]
        pub description: Option<String>,
        #[serde(default)]
        pub tags: Option<Vec<String>>,
        #[serde(default)]
        pub languages: Option<Vec<String>>,
        #[serde(default)]
        pub engine: Option<FeaturedEngine>,
        /// `""` clears it.
        #[serde(default)]
        pub game_version: Option<String>,
        #[serde(default)]
        pub requires_rustic_edition: Option<bool>,
        /// `""` clears it.
        #[serde(default)]
        pub website: Option<String>,
        /// `""` clears it.
        #[serde(default)]
        pub discord: Option<String>,
        #[serde(default)]
        pub visibility: Option<ContentVisibility>,
        #[serde(default)]
        pub icon_upload: Option<Uuid>,
        #[serde(default)]
        pub banner_upload: Option<Uuid>,
        /// Removes the icon (ignored with `icon_upload`).
        #[serde(default)]
        pub remove_icon: bool,
        /// Removes the banner (ignored with `banner_upload`).
        #[serde(default)]
        pub remove_banner: bool,
    }

    pub struct ServerListingReportRequest {
        pub reason: ReportReason,
        #[serde(default)]
        pub details: Option<String>,
    }

    pub struct ServerListingHideRequest {
        /// Shown to the owner (3-1000 characters).
        pub reason: String,
    }

    pub struct ServerListingFeatureRequest {
        /// Lower first (0-1000).
        #[serde(default)]
        pub rank: i32,
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

// ---------------------------------------------------------------- Discord (ADR 0167)

wire! {
    /// A Discord account linked to the caller's Rustic account (identity only: it never signs in).
    pub struct DiscordLink {
        /// Discord user id (a snowflake, as a string).
        pub discord_id: String,
        pub username: String,
        /// Display name on Discord, when set.
        pub global_name: Option<String>,
        /// Discord CDN avatar URL, when the account has one.
        pub avatar_url: Option<String>,
        pub linked_at: DateTime<Utc>,
    }

    /// `GET /v1/me/discord`.
    pub struct DiscordAccount {
        /// `false` while the server has no Discord OAuth2 client configured (linking disabled).
        pub available: bool,
        pub link: Option<DiscordLink>,
        /// Invite to the Rustic Discord server, when configured.
        pub invite_url: Option<String>,
    }

    /// Step 1: the website sends the browser to `authorize_url` (Discord's consent screen).
    pub struct DiscordLinkStart {
        pub authorize_url: String,
        pub expires_at: DateTime<Utc>,
    }

    /// Step 2: the website's callback page posts what Discord returned.
    pub struct DiscordLinkFinishRequest {
        pub code: String,
        pub state: String,
    }

    /// The public snapshot of a marketplace project for the bot's feed (service token, scope
    /// `discord`). `postable` is the API's decision: public, listed, not taken down, at least one
    /// public version. When it is `false` the bot removes what it posted.
    pub struct DiscordContentSnapshot {
        pub postable: bool,
        /// `None` when nobody but the owner and staff may see the project.
        pub project: Option<ContentProject>,
        /// Public versions, newest first (at most 20).
        pub versions: Vec<ContentVersion>,
        /// The project's page on the website.
        pub page_url: Option<String>,
        /// `rustic://content/rustic/<id>`: opens the project in the launcher.
        pub launcher_url: Option<String>,
    }

    /// The public snapshot of a server listing (ADR 0158) for the bot's feed.
    pub struct DiscordServerSnapshot {
        pub postable: bool,
        pub listing: Option<ServerListing>,
        pub page_url: Option<String>,
        /// `rustic://server/<slug>`.
        pub launcher_url: Option<String>,
    }

    /// A store product for the bot's feed (active products only).
    pub struct DiscordStoreSnapshot {
        pub postable: bool,
        pub product: Option<Product>,
        pub page_url: Option<String>,
    }

    /// A public project in a Discord profile card.
    pub struct DiscordProjectSummary {
        pub id: String,
        pub kind: ContentKind,
        pub name: String,
        pub downloads: i64,
        pub verified: bool,
        pub page_url: String,
    }

    /// A public server listing in a Discord profile card.
    pub struct DiscordServerSummary {
        pub slug: String,
        pub name: String,
        pub address: String,
        pub players_online: Option<i32>,
        pub page_url: String,
    }

    /// Public statistics of an account's published content and servers.
    pub struct DiscordStats {
        pub projects: i64,
        pub versions: i64,
        pub downloads: i64,
        pub servers: i64,
        /// Players online right now on the account's public servers (last ping).
        pub players_online: i64,
        /// Public projects per kind (`mod`, `modpack`...), most first.
        pub projects_by_kind: Vec<DiscordKindCount>,
    }

    pub struct DiscordKindCount {
        pub kind: ContentKind,
        pub count: i64,
    }

    /// `GET /v1/integrations/discord/profile`: only public data. A minor's Discord link and game
    /// name are never revealed to others; lookups by game name never resolve a minor.
    pub struct DiscordProfile {
        pub user: PublicUser,
        /// Linked game profile name (adults only, or the member looking at their own profile).
        pub minecraft_name: Option<String>,
        /// The Discord account this profile is linked to, when it may be shown.
        pub discord_id: Option<String>,
        /// The looked-up account is the asking Discord member's own.
        pub is_self: bool,
        pub stats: DiscordStats,
        /// Most downloaded public projects (at most 5).
        pub projects: Vec<DiscordProjectSummary>,
        /// Public server listings (at most 5).
        pub servers: Vec<DiscordServerSummary>,
        /// What the asking member may see of the Rustic profile (ADR 0168): the profile's
        /// visibility and section settings apply as on the website, the asking member being
        /// whoever their Discord account is linked to. `None` from an older API.
        #[serde(default)]
        pub access: Option<ProfileAccess>,
        /// `rustic-launcher.com/account/<username>`.
        #[serde(default)]
        pub profile_url: Option<String>,
        #[serde(default)]
        pub member_since: Option<DateTime<Utc>>,
        #[serde(default)]
        pub bio: Option<String>,
        #[serde(default)]
        pub badges: Option<ProfileBadges>,
        /// Play statistics, when the profile shows them to the asking member.
        #[serde(default)]
        pub play: Option<ProfileStats>,
        #[serde(default)]
        pub activity: Option<ProfileActivity>,
        #[serde(default)]
        pub friends_count: Option<i64>,
    }

    /// `GET /v1/integrations/discord/links/{discord_id}`: whether a Discord account is linked
    /// (role sync). Never says to which Rustic account.
    pub struct DiscordLinkState {
        pub discord_id: String,
        pub linked: bool,
    }
}

// ---------------------------------------------------------------- profiles (ADR 0168)

wire! {
    /// Who may open a profile page. Minors are never `public` (it is stored but shown as
    /// `friends`).
    #[derive(Copy, Eq, Hash, Default)]
    #[serde(rename_all = "snake_case")]
    pub enum ProfileVisibility {
        Public,
        #[default]
        Friends,
        Private,
    }

    /// Who sees one section of a visible profile.
    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum SectionVisibility {
        Public,
        Friends,
        Hidden,
    }

    /// Per-section choices. A section never reaches further than the profile itself.
    pub struct ProfileSections {
        /// Hours played, servers played, recently played, sessions.
        pub stats: SectionVisibility,
        /// Online now / last seen and the current activity.
        pub activity: SectionVisibility,
        pub friends: SectionVisibility,
        /// The account's public server directory listings.
        pub servers: SectionVisibility,
        /// The account's public marketplace projects.
        pub content: SectionVisibility,
        /// Owned and equipped cosmetics.
        pub locker: SectionVisibility,
        /// The linked game profile name (never shown to others for a minor).
        pub game_name: SectionVisibility,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum AvatarKind {
        /// One of the default heads, chosen or assigned from the account id.
        Default,
        /// The head (face + hat layer) of a skin the player uploaded.
        Skin,
    }

    pub struct AvatarInfo {
        pub url: Option<String>,
        pub kind: AvatarKind,
        /// The default head in use (1-20), when `kind` is `default`.
        pub default_id: Option<i32>,
        /// `false` while the player never chose: the head comes from the account id.
        pub chosen: bool,
    }

    pub struct DefaultAvatar {
        pub id: i32,
        pub url: String,
    }

    /// `GET /v1/me/settings`: the account settings stored server-side and shared by the website
    /// and the launcher (ADR 0168). Display name, country and language stay on `/v1/me`.
    pub struct AccountSettings {
        /// As chosen.
        pub visibility: ProfileVisibility,
        pub sections: ProfileSections,
        /// What applies (a minor's `public` reads `friends`).
        pub effective_visibility: ProfileVisibility,
        pub effective_sections: ProfileSections,
        /// `false` for minors: `public` is refused (`public_not_allowed`).
        pub public_allowed: bool,
        /// The launcher reports play sessions for the profile's statistics (default on).
        pub play_stats: bool,
        /// Short text on the profile (at most 160 characters).
        pub bio: String,
        /// Product news by email (opt-in, default off).
        pub email_news: bool,
        /// Staff only: show the staff role on the profile (default on).
        pub show_role: bool,
        pub avatar: AvatarInfo,
    }

    /// `PATCH /v1/me/settings`: only the fields present change.
    pub struct UpdateAccountSettingsRequest {
        #[serde(default)]
        pub visibility: Option<ProfileVisibility>,
        #[serde(default)]
        pub sections: Option<ProfileSections>,
        #[serde(default)]
        pub play_stats: Option<bool>,
        #[serde(default)]
        pub bio: Option<String>,
        #[serde(default)]
        pub email_news: Option<bool>,
        #[serde(default)]
        pub show_role: Option<bool>,
    }

    /// `PUT /v1/me/avatar`: one of the default heads.
    pub struct ChooseAvatarRequest {
        pub default_id: i32,
    }

    /// `POST /v1/me/presence-icons` (launcher ADR 0173): a Discord Rich Presence small image (the
    /// server's favicon or the world's icon) as the CDN serves it: a 64x64 PNG named by its
    /// SHA-256, the same for every player who sent the same picture.
    pub struct PresenceIcon {
        /// SHA-256 (hex) of the stored file.
        pub sha256: String,
        /// `https://cdn…/presence/<sha256>.png`.
        pub url: String,
    }

    /// What the viewer was allowed to see.
    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum ProfileAccess {
        /// The profile (each section by its own rule).
        Full,
        /// Only the card: the profile is for friends and the viewer is not one.
        FriendsOnly,
        /// Only the card: the profile is private.
        Private,
    }

    pub struct ProfileBadges {
        pub official: bool,
        /// At least one verified marketplace project.
        pub verified_creator: bool,
        /// `moderator` or `admin`, when the staff member shows it.
        pub staff_role: Option<Role>,
    }

    /// One server in the statistics. Servers that are not public directory listings are shown
    /// to others as one "other servers" line (`listing` empty); the owner also sees `address`.
    pub struct ProfileServerStat {
        /// The public directory listing this address belongs to (slug, name, icon).
        pub listing: Option<ProfilePlace>,
        /// Only for the owner of the profile (their own data), never for others.
        pub address: Option<String>,
        pub seconds: i64,
        pub sessions: i32,
        pub last_played_at: DateTime<Utc>,
    }

    /// A public directory listing named on a profile.
    pub struct ProfilePlace {
        pub slug: String,
        pub name: String,
        pub icon_url: Option<String>,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum RecentKind {
        Server,
        /// A world on the player's PC (never named).
        Singleplayer,
    }

    pub struct ProfileRecent {
        pub kind: RecentKind,
        /// The server's public listing; `None` for a private server or singleplayer.
        pub listing: Option<ProfilePlace>,
        pub last_played_at: DateTime<Utc>,
    }

    pub struct ProfileStats {
        pub total_seconds: i64,
        pub sessions: i32,
        pub singleplayer_seconds: i64,
        /// Most played first (at most 8 listed servers, then one line for the others).
        pub servers: Vec<ProfileServerStat>,
        /// Most recent first (at most 6).
        pub recent: Vec<ProfileRecent>,
        pub first_played_at: Option<DateTime<Utc>>,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum ProfileActivityKind {
        InLauncher,
        InGame,
        Hosting,
    }

    pub struct ProfileActivity {
        pub online: bool,
        pub kind: Option<ProfileActivityKind>,
        /// The public directory listing the player is on, when they share their server.
        pub listing: Option<ProfilePlace>,
        /// On a server that is not a public listing (or not shared): "on a server".
        pub on_server: bool,
        /// Last time the player was online (to the minute).
        pub last_seen_at: Option<DateTime<Utc>>,
    }

    pub struct ProfileFriends {
        pub count: i64,
        /// Friends the viewer may see the profile of (at most 24).
        pub users: Vec<PublicUser>,
    }

    pub struct ProfileServer {
        pub slug: String,
        pub name: String,
        pub address: String,
        pub summary: String,
        pub icon_url: Option<String>,
        pub online: Option<bool>,
        pub players_online: Option<i32>,
        pub verified: bool,
    }

    pub struct ProfileProject {
        pub id: String,
        pub kind: ContentKind,
        pub name: String,
        pub summary: String,
        pub icon_url: Option<String>,
        pub downloads: i64,
        pub verified: bool,
    }

    pub struct ProfileCosmetic {
        pub id: Uuid,
        pub name: String,
        pub slot: CosmeticSlot,
        pub equipped: bool,
    }

    pub struct ProfileLocker {
        pub owned_count: i64,
        /// Equipped first (at most 48).
        pub items: Vec<ProfileCosmetic>,
    }

    /// `GET /v1/profiles/{username}`: the page `rustic-launcher.com/account/<username>`. Every
    /// section is `None` when the viewer may not see it.
    pub struct PublicProfile {
        pub user: PublicUser,
        pub access: ProfileAccess,
        pub is_self: bool,
        pub is_friend: bool,
        /// Search engines may index the page (a public profile seen by anyone).
        pub indexable: bool,
        pub bio: Option<String>,
        pub member_since: Option<DateTime<Utc>>,
        pub badges: Option<ProfileBadges>,
        pub game_name: Option<String>,
        pub stats: Option<ProfileStats>,
        pub activity: Option<ProfileActivity>,
        pub friends: Option<ProfileFriends>,
        pub servers: Option<Vec<ProfileServer>>,
        pub content: Option<Vec<ProfileProject>>,
        pub locker: Option<ProfileLocker>,
        /// The owner's own view: what applies to others.
        pub settings: Option<AccountSettings>,
    }

    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum PlayPlaceKind {
        /// The title screen and the other menus outside a world.
        Menu,
        Singleplayer,
        Server,
    }

    /// `POST /v1/me/play-sessions`: the game started, or moved (menu, a world, a server). The
    /// cloud times everything itself; the launcher only says where the player is.
    pub struct PlaySessionReport {
        /// The launcher's id for this run of the game (one per launch).
        pub session_id: Uuid,
        /// `rustic` (the Rustic edition) or `horizon`.
        pub edition: String,
        #[serde(default)]
        pub game_version: Option<String>,
        pub place: PlayPlaceKind,
        /// The server's address as typed, for `server` (LAN and private addresses are not kept).
        #[serde(default)]
        pub server_address: Option<String>,
    }

    pub struct PlaySessionState {
        /// `false` when the account turned play statistics off: nothing was recorded.
        pub recording: bool,
    }

    /// `GET /v1/me/play-stats`: the owner's own statistics, with every address.
    pub struct MyPlayStats {
        pub enabled: bool,
        pub stats: ProfileStats,
        /// Raw sessions are kept 90 days, the totals until erased.
        pub raw_sessions: i64,
    }
}

// ---------------------------------------------------------------- contact form

wire! {
    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum ContactCategory {
        Support,
        Account,
        /// Purchases and billing.
        Billing,
        Partnership,
        Press,
        /// Report abuse.
        Abuse,
        Other,
    }

    /// `POST /v1/contact` (the website's contact page).
    pub struct ContactRequest {
        pub name: String,
        pub email: String,
        pub category: ContactCategory,
        pub subject: String,
        pub message: String,
        /// Optional link (a screenshot, a page).
        #[serde(default)]
        pub link: Option<String>,
        /// The sender agreed to the processing of their message (required).
        pub consent: bool,
        #[serde(default)]
        pub locale: Option<String>,
        /// Honeypot: a hidden field people leave empty.
        #[serde(default)]
        pub website: Option<String>,
        /// Milliseconds between the form showing and its sending.
        #[serde(default)]
        pub elapsed_ms: Option<i64>,
    }

    pub struct ContactAccepted {
        /// `None` when the message was dropped as spam (answered the same way).
        pub id: Option<Uuid>,
    }

    /// The staff notice the Discord bot posts for a contact message (no email address).
    pub struct DiscordContactNotice {
        /// Short reference, also in the emails' subjects.
        pub reference: String,
        pub category: ContactCategory,
        pub name: String,
        pub subject: String,
        /// The first 300 characters.
        pub excerpt: String,
        /// Sent by a signed-in member.
        pub signed_in: bool,
        pub created_at: DateTime<Utc>,
    }
}

// ---------------------------------------------------------------- back-office (ADR 0169)

wire! {
    /// An account as the back-office names it (staff only: carries the email).
    pub struct AdminUserRef {
        pub id: Uuid,
        pub username: String,
        pub display_name: String,
        pub email: Option<String>,
        pub avatar_url: Option<String>,
    }

    /// A row of the accounts table.
    pub struct AdminUserRow {
        pub id: Uuid,
        pub username: String,
        pub display_name: String,
        pub email: Option<String>,
        pub email_verified: bool,
        pub role: Role,
        /// `active`, `pending_deletion`.
        pub status: String,
        /// The official Rustic publisher (ADR 0166): never editable here.
        pub official: bool,
        pub country: Option<String>,
        pub locale: Option<String>,
        pub avatar_url: Option<String>,
        pub created_at: DateTime<Utc>,
        pub last_online_at: Option<DateTime<Utc>>,
        /// `banned` (permanent), `suspended` (until a date), `chat` (chat restricted) or null.
        pub restriction: Option<String>,
        pub restricted_until: Option<DateTime<Utc>>,
        pub is_minor: bool,
        pub totp_enabled: bool,
    }

    pub struct AdminUserPage {
        pub items: Vec<AdminUserRow>,
        pub total: i64,
    }

    pub struct AdminSession {
        pub id: Uuid,
        pub device_name: Option<String>,
        pub user_agent: Option<String>,
        pub ip: Option<String>,
        pub created_at: DateTime<Utc>,
        pub last_seen_at: DateTime<Utc>,
    }

    /// A linked identity: `microsoft`, `discord` or `game` (a verified game profile).
    pub struct AdminLinkedAccount {
        pub provider: String,
        pub label: Option<String>,
        pub linked_at: DateTime<Utc>,
    }

    pub struct AdminSpend {
        pub currency: String,
        pub total_minor: i64,
    }

    pub struct AdminPlaySummary {
        pub total_seconds: i64,
        pub sessions: i32,
        pub last_played_at: Option<DateTime<Utc>>,
    }

    pub struct AdminGuardianInfo {
        pub is_minor: bool,
        /// Below the digital consent age of the account's country without a linked guardian.
        pub needs_guardian: bool,
        pub age: Option<u32>,
        pub guardian: Option<AdminUserRef>,
        pub children: Vec<AdminUserRef>,
        pub approvals_pending: i64,
    }

    pub struct StaffNote {
        pub id: Uuid,
        pub author: Option<AdminUserRef>,
        pub body: String,
        pub created_at: DateTime<Utc>,
    }

    pub struct StaffNoteRequest {
        pub body: String,
    }

    /// Everything staff need about one account.
    pub struct AdminUserSheet {
        pub user: AdminUserRow,
        pub date_of_birth: Option<NaiveDate>,
        pub has_password: bool,
        pub sessions: Vec<AdminSession>,
        pub linked: Vec<AdminLinkedAccount>,
        pub orders: Vec<AdminOrderRow>,
        pub orders_count: i64,
        pub spend: Vec<AdminSpend>,
        pub projects: Vec<AdminProjectHit>,
        pub listings: Vec<AdminServerHit>,
        pub reports_against: i64,
        pub reports_filed: i64,
        pub bans: Vec<Ban>,
        pub friends: i64,
        pub play: Option<AdminPlaySummary>,
        pub deletion_scheduled_for: Option<DateTime<Utc>>,
        pub gdpr_requests: Vec<GdprRequest>,
        pub guardian: AdminGuardianInfo,
        pub notes: Vec<StaffNote>,
    }

    pub struct AdminSuspendRequest {
        pub reason: String,
        /// End of the suspension (future).
        pub until: DateTime<Utc>,
    }

    pub struct AdminReasonRequest {
        pub reason: String,
    }

    pub struct AdminRoleRequest {
        pub role: Role,
        pub reason: String,
    }

    // ------------------------------------------------------------ search

    pub struct AdminProjectHit {
        pub id: String,
        pub slug: String,
        pub name: String,
        pub kind: String,
        pub taken_down: bool,
    }

    pub struct AdminServerHit {
        pub id: Uuid,
        pub slug: String,
        pub name: String,
        pub address: String,
        pub hidden: bool,
    }

    pub struct AdminOrderHit {
        pub id: Uuid,
        pub status: String,
        pub total_minor: i64,
        pub currency: String,
        pub created_at: DateTime<Utc>,
        pub username: Option<String>,
    }

    /// The top bar's search: accounts, projects, servers and (admins) orders.
    pub struct AdminSearchResult {
        pub users: Vec<AdminUserRow>,
        pub projects: Vec<AdminProjectHit>,
        pub servers: Vec<AdminServerHit>,
        pub orders: Vec<AdminOrderHit>,
    }

    // ------------------------------------------------------------ payments

    pub struct AdminOrderRow {
        pub id: Uuid,
        pub status: String,
        pub currency: String,
        pub country: String,
        pub subtotal_minor: i64,
        pub discount_minor: i64,
        pub wallet_applied_minor: i64,
        pub total_minor: i64,
        pub created_at: DateTime<Utc>,
        pub paid_at: Option<DateTime<Utc>>,
        pub refunded_at: Option<DateTime<Utc>>,
        /// Null once anonymised (account deleted).
        pub user: Option<AdminUserRef>,
        pub anonymized: bool,
        pub products: Vec<String>,
        pub stripe_checkout_session_id: Option<String>,
        pub stripe_payment_intent_id: Option<String>,
        pub stripe_invoice_id: Option<String>,
        pub invoice_url: Option<String>,
        /// Refunded so far (Stripe's cumulative amount; the whole total once `refunded`).
        #[serde(default)]
        pub refunded_minor: i64,
    }

    pub struct AdminOrderPage {
        pub items: Vec<AdminOrderRow>,
        pub total: i64,
        /// Sum of `total_minor` of the paid orders matching the filters, per currency.
        pub paid_totals: Vec<AdminSpend>,
    }

    pub struct AdminOrderLine {
        pub product_id: Uuid,
        pub name: String,
        pub unit_price_minor: i64,
    }

    /// One step of an order's history: `order` (its own dates), `stripe` (a webhook event as
    /// recorded) or `audit` (an audit row about it).
    pub struct OrderTimelineEvent {
        pub at: DateTime<Utc>,
        pub source: String,
        pub kind: String,
        #[cfg_attr(feature = "openapi", schema(value_type = Object))]
        pub detail: serde_json::Value,
    }

    pub struct AdminOrderDetail {
        pub order: AdminOrderRow,
        pub lines: Vec<AdminOrderLine>,
        pub promo_code: Option<String>,
        pub creator_code: Option<String>,
        pub consent_immediate_execution: bool,
        pub consent_withdrawal_waiver: bool,
        pub consent_wording_version: String,
        pub consent_recorded_at: DateTime<Utc>,
        pub timeline: Vec<OrderTimelineEvent>,
        /// Refunds of the order, oldest first (ADR 0171).
        #[serde(default)]
        pub refunds: Vec<AdminRefund>,
        /// What can still be refunded (0 unless paid).
        #[serde(default)]
        pub refundable_minor: i64,
    }

    // ------------------------------------------------------------ audit

    pub struct AuditPage {
        pub items: Vec<AuditEntry>,
        pub total: i64,
    }

    /// Distinct values for the audit filters.
    pub struct AuditFacets {
        pub actions: Vec<String>,
        pub target_types: Vec<String>,
        /// How long rows are kept (days); null = kept (accounts' ids are erased on deletion).
        pub retention_days: Option<i64>,
    }

    // ------------------------------------------------------------ GDPR

    /// A data-subject request in the register (export, deletion...), self-service or received.
    pub struct GdprRequest {
        pub id: Uuid,
        pub user: Option<AdminUserRef>,
        /// The account's handle when recorded (`deleted account` after the purge).
        pub username: String,
        /// `export`, `deletion`, `rectification`, `other`.
        pub kind: String,
        /// `pending`, `processing`, `done`, `refused`, `cancelled`.
        pub status: String,
        /// `self_service`, `email`, `contact_form`, `staff`.
        pub source: String,
        pub requested_at: DateTime<Utc>,
        /// The legal deadline (one month, GDPR art. 12(3)).
        pub due_at: DateTime<Utc>,
        pub handled_by: Option<AdminUserRef>,
        pub handled_at: Option<DateTime<Utc>>,
        pub note: Option<String>,
        pub overdue: bool,
    }

    pub struct GdprRequestCreate {
        pub user_id: Uuid,
        pub kind: String,
        pub source: String,
        pub note: Option<String>,
    }

    pub struct GdprRequestUpdate {
        pub status: String,
        pub note: Option<String>,
    }

    pub struct ScheduledDeletion {
        pub user: AdminUserRef,
        pub scheduled_for: DateTime<Utc>,
        pub requested_at: Option<DateTime<Utc>>,
    }

    /// A minor below the digital consent age with no guardian linked yet (or no birth date).
    pub struct PendingMinor {
        pub user: AdminUserRef,
        pub age: Option<u32>,
        pub country: Option<String>,
        pub created_at: DateTime<Utc>,
    }

    pub struct GdprOverview {
        pub requests: Vec<GdprRequest>,
        pub deletions: Vec<ScheduledDeletion>,
        pub minors: Vec<PendingMinor>,
        /// Purchases waiting for a guardian's decision.
        pub approvals_pending: i64,
        pub grace_days: i64,
    }

    // ------------------------------------------------------------ statistics

    pub struct StatPoint {
        pub day: NaiveDate,
        pub value: f64,
    }

    /// A daily series (`signups`, `active_users`, `play_hours`, `revenue_eur`...).
    pub struct StatSeries {
        pub key: String,
        pub points: Vec<StatPoint>,
    }

    /// A headline number for the range, with the same measure over the previous range.
    pub struct StatKpi {
        pub key: String,
        pub value: f64,
        pub previous: Option<f64>,
        /// `count`, `hours` or a currency code for amounts in minor units.
        pub unit: String,
    }

    pub struct StatItem {
        pub key: String,
        pub value: f64,
    }

    /// A split of the range (`signup_country`, `downloads_os`, `launcher_version`...).
    pub struct StatBreakdown {
        pub key: String,
        pub items: Vec<StatItem>,
    }

    pub struct StatsReport {
        pub from: NaiveDate,
        pub to: NaiveDate,
        pub generated_at: DateTime<Utc>,
        pub kpis: Vec<StatKpi>,
        pub series: Vec<StatSeries>,
        pub breakdowns: Vec<StatBreakdown>,
        /// False while no subscription product exists (Rustic Premium is a waitlist).
        pub subscriptions_offered: bool,
    }

    // ------------------------------------------------------------ anonymous counters

    /// The launcher's daily anonymous ping (no account, no install id; ADR 0169).
    pub struct LauncherPing {
        pub version: String,
        /// `windows`, `macos`, `linux`.
        pub os: String,
        /// `x86_64`, `aarch64`.
        #[serde(default)]
        pub arch: Option<String>,
        /// `stable`, `beta`, `dev`.
        #[serde(default)]
        pub channel: Option<String>,
    }
}

// ---------------------------------------------------------------- back-office, part B (ADR 0171)

wire! {
    // ------------------------------------------------------------ system health

    /// One service as the API sees it.
    pub struct ServiceHealth {
        /// `api`, `realtime`, `relay`, `discord-bot`.
        pub name: String,
        /// `ok`, `degraded`, `down`, `unconfigured` (no health address for it).
        pub status: String,
        pub latency_ms: Option<i64>,
        pub detail: Option<String>,
    }

    pub struct DatabaseHealth {
        pub ok: bool,
        pub latency_ms: Option<i64>,
        pub size_bytes: Option<i64>,
        /// The newest migration applied to the database.
        pub migration_version: Option<i64>,
        /// The newest migration this API embeds.
        pub migration_latest: i64,
        pub connections: Option<i64>,
    }

    pub struct CdnSection {
        pub name: String,
        pub bytes: i64,
        pub files: i64,
    }

    pub struct CdnHealth {
        /// The CDN folder is readable by the API.
        pub readable: bool,
        pub used_bytes: Option<i64>,
        pub files: Option<i64>,
        /// The walk stopped at its file limit: the figures are a lower bound.
        pub truncated: bool,
        pub disk_total_bytes: Option<i64>,
        pub disk_free_bytes: Option<i64>,
        pub sections: Vec<CdnSection>,
        /// When these figures were measured (cached for 10 minutes).
        pub measured_at: DateTime<Utc>,
    }

    /// Answers of this API process (counted in memory since it started).
    pub struct ErrorCounts {
        pub since: DateTime<Utc>,
        pub requests_last_hour: i64,
        pub server_errors_last_hour: i64,
        pub server_errors_last_24h: i64,
        pub client_errors_last_hour: i64,
        pub rate_limited_last_hour: i64,
        /// Stripe webhook events received but not processed (last 24 hours).
        pub stripe_events_unprocessed_24h: i64,
    }

    pub struct EmailHealth {
        /// SMTP is configured (otherwise mails stay in memory).
        pub configured: bool,
        pub sent: i64,
        pub failed: i64,
        pub last_sent_at: Option<DateTime<Utc>>,
        pub last_failure_at: Option<DateTime<Utc>>,
        pub last_error: Option<String>,
        pub campaigns_sending: i64,
        pub campaign_failures_24h: i64,
    }

    pub struct DiscordFeedHealth {
        /// Outbox changes the bot has not consumed yet.
        pub pending: i64,
        pub oldest_pending_at: Option<DateTime<Utc>>,
        /// Age of the oldest change not consumed (0 when caught up).
        pub lag_seconds: i64,
        pub cursor_updated_at: Option<DateTime<Utc>>,
        pub last_post_at: Option<DateTime<Utc>>,
        /// Back-office actions waiting for the bot.
        pub pending_actions: i64,
    }

    pub struct SystemHealth {
        pub checked_at: DateTime<Utc>,
        pub api_version: String,
        pub api_started_at: DateTime<Utc>,
        pub services: Vec<ServiceHealth>,
        pub database: DatabaseHealth,
        pub cdn: CdnHealth,
        pub errors: ErrorCounts,
        pub email: EmailHealth,
        pub discord: DiscordFeedHealth,
        /// Stripe keys are configured.
        pub payments_configured: bool,
    }

    // ------------------------------------------------------------ releases

    pub struct ReleaseAdoption {
        /// Launchers of this version that pinged yesterday (UTC).
        pub installs_yesterday: i64,
        /// Their share of yesterday's pings, 0 to 1.
        pub share_yesterday: f64,
        /// Pings of the last 7 days (an install pings at most once a day).
        pub pings_7d: i64,
    }

    pub struct AdminRelease {
        /// `launcher`, `edition`.
        pub product: String,
        pub version: String,
        pub published_at: Option<DateTime<Utc>>,
        pub notes: String,
        pub prerelease: bool,
        pub pulled_at: Option<DateTime<Utc>>,
        pub pulled_by: Option<AdminUserRef>,
        pub pull_reason: Option<String>,
        /// Listed by the live manifest.
        pub live: bool,
        /// The version updates install now (the launcher feed's version, the edition channel's
        /// newest stable build).
        pub current: bool,
        pub first_seen_at: DateTime<Utc>,
        pub last_seen_at: DateTime<Utc>,
        /// From the launcher's anonymous daily ping; none for edition builds (not reported).
        pub adoption: Option<ReleaseAdoption>,
    }

    pub struct AdminReleases {
        pub launcher: Vec<AdminRelease>,
        pub edition: Vec<AdminRelease>,
        /// `files` (the API reads the CDN folder and can act), `cdn` (read over HTTPS only,
        /// read-only), `none` (no manifest reachable).
        pub source: String,
        /// Pull / restore / pre-release act (the manifests are writable by the API).
        pub writable: bool,
        pub synced_at: DateTime<Utc>,
        pub problems: Vec<String>,
        /// Launcher pings yesterday, all versions.
        pub launcher_pings_yesterday: i64,
    }

    pub struct ReleasePullRequest {
        pub reason: String,
    }

    pub struct ReleasePrereleaseRequest {
        pub prerelease: bool,
        pub reason: String,
    }

    // ------------------------------------------------------------ banners

    pub struct BannerText {
        pub title: String,
        /// Markdown (the website's renderer: no HTML).
        pub body: String,
    }

    pub struct AdminBanner {
        pub id: Uuid,
        /// `site`, `launcher`, `both`.
        pub surface: String,
        /// `info`, `success`, `warning`, `critical`.
        pub severity: String,
        /// Per language (`en`, `fr`, `pt-BR`...).
        pub texts: std::collections::BTreeMap<String, BannerText>,
        pub fallback_en: bool,
        pub dismissible: bool,
        pub starts_at: Option<DateTime<Utc>>,
        pub ends_at: Option<DateTime<Utc>>,
        pub active: bool,
        /// Shown right now (active and inside its window).
        pub live: bool,
        pub created_by: Option<AdminUserRef>,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
    }

    pub struct BannerInput {
        pub surface: String,
        pub severity: String,
        pub texts: std::collections::BTreeMap<String, BannerText>,
        #[serde(default = "crate::types::yes")]
        pub fallback_en: bool,
        #[serde(default = "crate::types::yes")]
        pub dismissible: bool,
        #[serde(default)]
        pub starts_at: Option<DateTime<Utc>>,
        #[serde(default)]
        pub ends_at: Option<DateTime<Utc>>,
        #[serde(default = "crate::types::yes")]
        pub active: bool,
    }

    /// A banner as the website or the launcher shows it, in the asked language.
    pub struct PublicBanner {
        pub id: Uuid,
        pub severity: String,
        /// The language of the text (the asked one, or `en` as the fallback).
        pub lang: String,
        pub title: String,
        pub body: String,
        pub dismissible: bool,
        pub ends_at: Option<DateTime<Utc>>,
        /// Changes when the banner is edited: a dismissal is remembered per id and version.
        pub updated_at: DateTime<Utc>,
    }

    // ------------------------------------------------------------ feature flags

    pub struct FeatureFlag {
        /// `signups`, `store`, `marketplace_publishing`, `native_mods`, `server_listings`.
        pub key: String,
        pub enabled: bool,
        /// Exposed to the clients by `GET /v1/flags`.
        pub public: bool,
        pub note: String,
        /// What the API refuses while it is off.
        pub gates: Vec<String>,
        pub updated_by: Option<AdminUserRef>,
        pub updated_at: DateTime<Utc>,
    }

    pub struct UpdateFeatureFlagRequest {
        pub enabled: bool,
        #[serde(default)]
        pub public: Option<bool>,
        #[serde(default)]
        pub note: Option<String>,
        pub reason: String,
    }

    /// The public flags (`true`: available).
    pub struct PublicFlags {
        pub flags: std::collections::BTreeMap<String, bool>,
    }

    // ------------------------------------------------------------ promo codes

    pub struct AdminPromoCode {
        pub id: Uuid,
        pub code: String,
        pub percent_off: Option<i32>,
        pub amount_off_minor: Option<i64>,
        pub currency: Option<Currency>,
        pub max_redemptions: Option<i32>,
        /// Orders holding the code (pending, awaiting a guardian or paid).
        pub redeemed_count: i32,
        pub per_user_limit: i32,
        pub min_subtotal_minor: Option<i64>,
        pub starts_at: Option<DateTime<Utc>>,
        pub valid_until: Option<DateTime<Utc>>,
        pub active: bool,
        /// `active`, `scheduled`, `expired`, `exhausted`, `disabled`.
        pub state: String,
        pub note: String,
        pub paid_orders: i64,
        /// Discounts granted on paid orders, per currency.
        pub discount_given: Vec<AdminSpend>,
        pub created_by: Option<AdminUserRef>,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
    }

    pub struct AdminPromoPage {
        pub items: Vec<AdminPromoCode>,
        pub total: i64,
    }

    /// The editable part of a code (the code, its discount and currency are fixed once created:
    /// orders refer to them).
    pub struct UpdatePromoCodeRequest {
        pub active: bool,
        #[serde(default)]
        pub max_redemptions: Option<i32>,
        pub per_user_limit: i32,
        #[serde(default)]
        pub min_subtotal_minor: Option<i64>,
        #[serde(default)]
        pub starts_at: Option<DateTime<Utc>>,
        #[serde(default)]
        pub valid_until: Option<DateTime<Utc>>,
        #[serde(default)]
        pub note: String,
    }

    // ------------------------------------------------------------ refunds

    pub struct AdminRefund {
        pub id: Uuid,
        pub order_id: Uuid,
        pub amount_minor: i64,
        pub currency: String,
        pub full_refund: bool,
        pub reason: Option<String>,
        /// `admin` (the back-office) or `stripe` (seen in a webhook: the Stripe dashboard).
        pub source: String,
        /// `pending`, `requires_action`, `succeeded`, `failed`, `canceled`.
        pub status: String,
        pub stripe_refund_id: Option<String>,
        pub failure_reason: Option<String>,
        pub created_by: Option<AdminUserRef>,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
    }

    pub struct AdminRefundRow {
        pub refund: AdminRefund,
        pub buyer: Option<AdminUserRef>,
        pub order_total_minor: i64,
        pub order_status: String,
    }

    pub struct AdminRefundPage {
        pub items: Vec<AdminRefundRow>,
        pub total: i64,
        /// Succeeded refunds of the filtered rows, per currency.
        pub refunded_totals: Vec<AdminSpend>,
    }

    pub struct RefundOrderRequest {
        /// Minor units; none: everything still refundable (a full refund, items revoked).
        #[serde(default)]
        pub amount_minor: Option<i64>,
        pub reason: String,
    }

    // ------------------------------------------------------------ bulk emails

    pub struct EmailCampaignCounts {
        pub total: i64,
        pub pending: i64,
        pub sent: i64,
        pub failed: i64,
        pub skipped: i64,
    }

    pub struct AdminEmailCampaign {
        pub id: Uuid,
        pub subject: String,
        /// Markdown, sent as plain text.
        pub body: String,
        pub locale: Option<String>,
        /// `draft`, `sending`, `sent`, `cancelled`.
        pub status: String,
        pub counts: EmailCampaignCounts,
        pub created_by: Option<AdminUserRef>,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
        pub tested_at: Option<DateTime<Utc>>,
        pub started_at: Option<DateTime<Utc>>,
        pub finished_at: Option<DateTime<Utc>>,
    }

    pub struct EmailLocaleCount {
        pub locale: Option<String>,
        pub count: i64,
    }

    /// Accounts a campaign would reach today (opted in, verified address, active).
    pub struct EmailAudience {
        pub opted_in: i64,
        pub by_locale: Vec<EmailLocaleCount>,
    }

    pub struct AdminEmailPage {
        pub items: Vec<AdminEmailCampaign>,
        pub total: i64,
        pub audience: EmailAudience,
        /// The worker's pace.
        pub per_minute: i64,
        /// SMTP is configured.
        pub delivers: bool,
    }

    pub struct EmailCampaignInput {
        pub subject: String,
        pub body: String,
        #[serde(default)]
        pub locale: Option<String>,
    }

    // ------------------------------------------------------------ Discord bot

    /// A moderation case. Discord ids are strings (they exceed JavaScript's safe integers).
    pub struct AdminDiscordCase {
        pub id: i64,
        pub action: String,
        pub target_id: Option<String>,
        /// The Rustic account linked to the target, if any.
        pub target_account: Option<AdminUserRef>,
        pub channel_id: Option<String>,
        pub moderator_id: String,
        pub reason: Option<String>,
        pub duration_secs: Option<i64>,
        pub expires_at: Option<DateTime<Utc>>,
        pub automatic: bool,
        #[cfg_attr(feature = "openapi", schema(value_type = Object))]
        pub details: serde_json::Value,
        pub created_at: DateTime<Utc>,
        pub edited_at: Option<DateTime<Utc>>,
        pub revoked_at: Option<DateTime<Utc>>,
        pub revoked_by: Option<AdminUserRef>,
        pub revoke_reason: Option<String>,
        /// Still in force (a ban not lifted, a mute not over, a warning not cleared).
        pub in_force: bool,
        /// Can be revoked from the back-office (bans, mutes and warnings, not yet revoked).
        pub revocable: bool,
    }

    pub struct AdminDiscordCasePage {
        pub items: Vec<AdminDiscordCase>,
        pub total: i64,
    }

    pub struct UpdateDiscordCaseRequest {
        pub reason: String,
    }

    pub struct RevokeDiscordCaseRequest {
        pub reason: String,
    }

    pub struct AdminDiscordTicket {
        pub id: i64,
        pub thread_id: String,
        pub opener_id: String,
        pub opener_name: Option<String>,
        pub opener_account: Option<AdminUserRef>,
        pub category: String,
        pub subject: String,
        /// `open`, `closed`.
        pub status: String,
        /// 1 Normal, 2 High, 3 Urgent.
        pub priority: i16,
        pub claimed_by: Option<String>,
        pub closed_by: Option<String>,
        pub close_reason: Option<String>,
        pub created_at: DateTime<Utc>,
        pub closed_at: Option<DateTime<Utc>>,
        pub transcripts: i64,
        pub last_transcript_url: Option<String>,
    }

    pub struct AdminDiscordTicketPage {
        pub items: Vec<AdminDiscordTicket>,
        pub total: i64,
    }

    /// Something the bot did: a feed post or its removal, an automatic sanction, a back-office
    /// action, a GitHub delivery.
    pub struct DiscordActivityItem {
        pub at: DateTime<Utc>,
        /// `post`, `post_removed`, `auto_case`, `admin_action`, `github`.
        pub kind: String,
        pub subject: Option<String>,
        #[cfg_attr(feature = "openapi", schema(value_type = Object))]
        pub detail: serde_json::Value,
    }

    pub struct DiscordActivityPage {
        pub items: Vec<DiscordActivityItem>,
        pub total: i64,
    }

    pub struct DiscordFeedToggles {
        pub content: bool,
        pub servers: bool,
        pub store: bool,
        pub releases: bool,
        pub github: bool,
    }

    /// An automatic sanction at a number of active warnings.
    pub struct DiscordThreshold {
        pub count: i32,
        /// `mute`, `kick`, `ban`.
        pub action: String,
        /// Mutes (up to 28 days) and temporary bans (up to 365 days); none for a kick or a
        /// permanent ban.
        #[serde(default)]
        pub duration_secs: Option<i64>,
    }

    pub struct DiscordActivity {
        /// `playing`, `watching`, `listening`, `competing`.
        pub kind: String,
        pub text: String,
    }

    pub struct DiscordBotSettings {
        pub feeds: DiscordFeedToggles,
        /// Empty: no automatic sanctions.
        pub warn_thresholds: Vec<DiscordThreshold>,
        /// The rotating activity, in order.
        pub activities: Vec<DiscordActivity>,
        /// Seconds each activity shows (30 to 3600).
        pub activity_every_secs: i32,
    }

    pub struct AdminDiscordSettings {
        /// Saved from the back-office (otherwise the bot runs on its environment and these are
        /// the built-in defaults).
        pub configured: bool,
        pub settings: DiscordBotSettings,
        pub updated_by: Option<AdminUserRef>,
        pub updated_at: Option<DateTime<Utc>>,
    }

    pub struct UpdateDiscordSettingsRequest {
        pub settings: DiscordBotSettings,
    }

    pub struct AdminDiscordOverview {
        pub linked_accounts: i64,
        pub open_tickets: i64,
        pub tickets_30d: i64,
        pub cases_30d: i64,
        pub automatic_cases_30d: i64,
        pub active_temp_bans: i64,
        pub active_warnings: i64,
        pub pending_actions: i64,
        pub posts_live: i64,
        pub last_post_at: Option<DateTime<Utc>>,
        /// The back-office's latest requests to the bot, newest first.
        pub recent_actions: Vec<DiscordAdminAction>,
        /// The outbox the feeds consume.
        pub feed: DiscordFeedHealth,
    }

    /// The settings the bot applies (`GET /v1/integrations/discord/settings`).
    pub struct DiscordServiceSettings {
        /// False: no row saved, the bot keeps its environment.
        pub configured: bool,
        pub settings: Option<DiscordBotSettings>,
        pub updated_at: Option<DateTime<Utc>>,
    }

    /// A back-office request the bot applies on Discord.
    pub struct DiscordAdminAction {
        pub id: i64,
        /// `unban`, `unmute`.
        pub kind: String,
        pub guild_id: String,
        pub target_id: String,
        pub case_id: Option<i64>,
        pub reason: String,
        pub requested_by_name: String,
        /// `pending`, `done`, `failed`.
        pub status: String,
        pub result_case_id: Option<i64>,
        pub error: Option<String>,
        pub created_at: DateTime<Utc>,
        pub done_at: Option<DateTime<Utc>>,
    }

    pub struct DiscordActionResult {
        pub ok: bool,
        #[serde(default)]
        pub error: Option<String>,
        /// The case the bot recorded for it.
        #[serde(default)]
        pub case_id: Option<i64>,
    }
}

/// `true`, for `#[serde(default = ...)]`.
pub fn yes() -> bool {
    true
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

// ---------------------------------------------------------------- voice (ADR 0176)

wire! {
    #[derive(Copy, Eq, Hash)]
    #[serde(rename_all = "snake_case")]
    pub enum VoiceRoomKind {
        /// The caller's party.
        Party,
        /// The proximity voice of the Rustic Server the caller is playing on.
        Server,
    }

    pub struct VoiceTokenRequest {
        pub room: VoiceRoomKind,
    }

    /// What the launcher needs to join a voice room on Rustic's SFU (LiveKit).
    pub struct VoiceToken {
        /// The SFU's WebSocket address (`wss://voice.rustic-launcher.com`).
        pub url: String,
        /// A LiveKit access token for `room` only, short-lived (join before `expires_at`).
        pub token: String,
        pub room: String,
        pub kind: VoiceRoomKind,
        /// The caller's identity in the room (their Rustic user id).
        pub identity: String,
        pub expires_at: DateTime<Utc>,
        /// Distance-based volume (always for a server room; the party's setting otherwise).
        pub proximity: bool,
        /// Blocks beyond which a speaker is silent.
        pub range: i32,
        /// The server's name (its owner's label), for a server room.
        #[serde(default)]
        pub server_name: Option<String>,
    }

    /// A Rustic Server registered for proximity voice (its owner's view).
    pub struct VoiceServer {
        pub id: Uuid,
        pub name: String,
        pub enabled: bool,
        pub created_at: DateTime<Utc>,
        pub last_seen_at: Option<DateTime<Utc>>,
        /// Players in the last roster the plugin sent (0 once it is stale).
        pub online: i64,
    }

    pub struct CreateVoiceServerRequest {
        /// 1-64 characters, shown to players in the launcher's voice panel.
        pub name: String,
    }

    pub struct UpdateVoiceServerRequest {
        #[serde(default)]
        pub name: Option<String>,
        #[serde(default)]
        pub enabled: Option<bool>,
    }

    /// The key is shown once: it goes in the plugin's `config.toml`.
    pub struct VoiceServerKey {
        pub server: VoiceServer,
        /// `rvk_` + 43 URL-safe characters.
        pub key: String,
    }

    /// `POST /v1/voice/servers/{id}/roster` from the proximity voice plugin (bearer = its key).
    pub struct VoiceRosterRequest {
        /// The Minecraft UUIDs online now (32 hex or dashed), at most 500.
        pub players: Vec<String>,
        /// The plugin's range in blocks (8-128, default 48).
        #[serde(default)]
        pub range: Option<i32>,
    }

    pub struct VoiceRosterAnswer {
        /// Players kept (well-formed UUIDs).
        pub accepted: i64,
        /// Seconds the roster stays valid without a new one.
        pub ttl_secs: i64,
        /// `false` when the owner disabled the server in the cloud: the plugin stops sending.
        pub enabled: bool,
    }
}

// ---------------------------------------------------------------- hero video (ADR 0179)

wire! {
    /// One encoding of the hero loop, on the CDN.
    pub struct HeroRendition {
        pub width: i32,
        pub height: i32,
        /// `av1`, `vp9` or `h264`.
        pub codec: String,
        /// The type with its codecs, for `<source type>`, `canPlayType` and
        /// `MediaSource.isTypeSupported`: `video/mp4; codecs="av01.0.08M.08"`.
        pub mime: String,
        /// Absolute URL on the CDN (an immutable, content-addressed folder).
        pub url: String,
        pub bytes: i64,
        /// Lowercase hex SHA-256 of the file (the launcher checks its cached copy against it).
        pub sha256: String,
    }

    /// A still of the loop's first frame (the `poster` and the reduced-motion picture).
    pub struct HeroPoster {
        pub width: i32,
        pub height: i32,
        /// `image/jpeg`, `image/webp` or `image/avif`.
        pub mime: String,
        pub url: String,
        pub bytes: i64,
        pub sha256: String,
    }

    /// Where the footage comes from and under which licence.
    pub struct HeroCredit {
        pub title: String,
        pub author: String,
        /// The page the clip was downloaded from.
        pub source_url: String,
        /// `Pexels License`, `CC0-1.0`...
        pub licence: String,
        #[serde(default)]
        pub licence_url: Option<String>,
    }

    /// The video set behind the launcher's Launchpad and the website's account pages.
    pub struct HeroMedia {
        /// The set's content hash: its folder `media/hero/<id>/` on the CDN.
        pub id: String,
        pub duration_ms: i64,
        /// Every rendition; the clients pick by codec support, then by size.
        pub renditions: Vec<HeroRendition>,
        pub posters: Vec<HeroPoster>,
        pub credit: HeroCredit,
        /// No choice saved in the back-office: the set built into the API.
        pub builtin: bool,
        pub updated_at: Option<DateTime<Utc>>,
    }

    /// The back-office's view of the hero video.
    pub struct AdminHeroMedia {
        pub active: HeroMedia,
        /// The id of the set built into the API (what "Restore the default" brings back).
        pub default_id: String,
        /// Sets found in the CDN folder (`media/hero/<id>/manifest.json`), newest first.
        pub available: Vec<String>,
        pub updated_by: Option<AdminUserRef>,
    }

    /// `PUT /v1/admin/media/hero`.
    pub struct HeroMediaUpdate {
        /// A set id (8 to 64 hexadecimal characters), its manifest URL on the CDN
        /// (`https://cdn…/media/hero/<id>/manifest.json`), or `default` for the built-in set.
        pub source: String,
        /// Why (3 to 500 characters), kept in the audit log.
        pub reason: String,
    }
}
