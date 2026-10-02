use crate::hud::HudViewport;
use crate::model::Setting;
use crate::safety::canonicalize_setting;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::de::{self, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{Cursor, Read};
use std::marker::PhantomData;

pub const PREFIX: &str = "PRS3:";
pub const COMPACT_PREFIX: &str = "PRS2:";
pub const LEGACY_PREFIX: &str = "PRS1:";
pub const MAX_CODE_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_UNCOMPRESSED_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_SETTINGS: usize = 10_000;
pub const POINTER_DICTIONARY: &[&str] = &[
    "/3D_SKINS/enabled",
    "/ARMORSTATUS/ARMORSTATUS_BOOTS_CHILD/position",
    "/ARMORSTATUS/ARMORSTATUS_BOOTS_CHILD/y",
    "/ARMORSTATUS/ARMORSTATUS_CHESTPLATE_CHILD/position",
    "/ARMORSTATUS/ARMORSTATUS_CHESTPLATE_CHILD/y",
    "/ARMORSTATUS/ARMORSTATUS_HELMET_CHILD/position",
    "/ARMORSTATUS/ARMORSTATUS_HELMET_CHILD/y",
    "/ARMORSTATUS/ARMORSTATUS_LEGGINGS_CHILD/position",
    "/ARMORSTATUS/ARMORSTATUS_LEGGINGS_CHILD/y",
    "/ARMORSTATUS/ARMORSTATUS_OFF_HAND_HELD_ITEM_CHILD/position",
    "/ARMORSTATUS/ARMORSTATUS_OFF_HAND_HELD_ITEM_CHILD/y",
    "/ARMORSTATUS/ARMORSTATUS_PROTECTION_CHILD/position",
    "/ARMORSTATUS/ARMORSTATUS_PROTECTION_CHILD/y",
    "/ARMORSTATUS/options/armorDamage",
    "/ARMORSTATUS/options/itemDamage",
    "/ARMORSTATUS/x",
    "/ARMORSTATUS/y",
    "/AUDIO_SUBTITLES/position",
    "/AUDIO_SUBTITLES/y",
    "/BLOCK_OUTLINE/enabled",
    "/BLOCK_OUTLINE/options/blockOutlineColor/chroma",
    "/BLOCK_OUTLINE/options/blockOutlineColor/value",
    "/BLOCK_OUTLINE/options/blockOutlineWidth",
    "/BLOCK_OUTLINE/options/blockOverlay",
    "/BOSSBAR/position",
    "/BOSSBAR/x",
    "/BOSSBAR/y",
    "/CHAT/options/copyChat",
    "/CHAT/options/stackMessages",
    "/CHAT/options/timeBasedStackMessagesTimeframe",
    "/CLOCK/options/backgroundColor/chroma",
    "/CLOCK/options/backgroundColor/value",
    "/CLOCK/position",
    "/CLOCK/x",
    "/CLOCK/y",
    "/COMBO/options/backgroundColor/chroma",
    "/COMBO/options/backgroundColor/value",
    "/COMBO/options/backgroundHeight",
    "/COMBO/position",
    "/COMBO/x",
    "/COMBO/y",
    "/COOLDOWNS/enabled",
    "/COOLDOWNS/position",
    "/COOLDOWNS/x",
    "/COOLDOWNS/y",
    "/COORDINATES/enabled",
    "/COORDINATES/options/backgroundColor/chroma",
    "/COORDINATES/options/backgroundColor/value",
    "/COORDINATES/options/showAxisLabels",
    "/COORDINATES/options/showC",
    "/COORDINATES/options/showDirection",
    "/COORDINATES/position",
    "/COORDINATES/x",
    "/COORDINATES/y",
    "/CPS/enabled",
    "/CPS/options/background",
    "/CPS/options/backgroundColor/chroma",
    "/CPS/options/backgroundColor/value",
    "/CPS/options/lineColor/value",
    "/CPS/options/rightClick",
    "/CPS/position",
    "/CPS/x",
    "/CPS/y",
    "/CROSSHAIR/CROSSHAIR_ENEMY/options/gridSize",
    "/CROSSHAIR/CROSSHAIR_FRIENDLY/options/gridSize",
    "/CROSSHAIR/CROSSHAIR_NORMAL/options/gridSize",
    "/CROSSHAIR/options/crosshairGap",
    "/CROSSHAIR/options/crosshairOutline",
    "/CROSSHAIR/options/crosshairSize",
    "/CROSSHAIR/options/help_box_open",
    "/DIRECTION_HUD/enabled",
    "/DIRECTION_HUD/options/useLegacyStyle",
    "/DIRECTION_HUD/position",
    "/DIRECTION_HUD/x",
    "/DIRECTION_HUD/y",
    "/F3_DISPLAY/default_game_info/options/background",
    "/F3_DISPLAY/default_game_info/options/labelColor/value",
    "/F3_DISPLAY/default_game_info/options/valueColor/value",
    "/F3_DISPLAY/default_pie_chart/options/background",
    "/F3_DISPLAY/default_pie_chart/options/labelColor/value",
    "/F3_DISPLAY/default_pie_chart/options/scale",
    "/F3_DISPLAY/default_pie_chart/options/valueColor/value",
    "/F3_DISPLAY/default_pie_chart/position",
    "/F3_DISPLAY/default_pie_chart/y",
    "/F3_DISPLAY/default_player_info/options/background",
    "/F3_DISPLAY/default_player_info/options/labelColor/value",
    "/F3_DISPLAY/default_player_info/options/valueColor/value",
    "/F3_DISPLAY/default_player_info/position",
    "/F3_DISPLAY/default_player_info/y",
    "/F3_DISPLAY/default_target_info/options/background",
    "/F3_DISPLAY/default_target_info/options/labelColor/value",
    "/F3_DISPLAY/default_target_info/options/valueColor/value",
    "/F3_DISPLAY/default_target_info/position",
    "/F3_DISPLAY/default_target_info/y",
    "/F3_DISPLAY/default_world_info/options/background",
    "/F3_DISPLAY/default_world_info/options/labelColor/value",
    "/F3_DISPLAY/default_world_info/options/valueColor/value",
    "/F3_DISPLAY/default_world_info/position",
    "/F3_DISPLAY/default_world_info/y",
    "/FOG/enabled",
    "/FOG/options/renderDistanceFogColorToggle",
    "/FOG/options/renderDistanceFogDensity",
    "/FOG/options/waterFogDensity",
    "/FPS/enabled",
    "/FPS/options/background",
    "/FPS/options/backgroundColor/chroma",
    "/FPS/options/backgroundColor/value",
    "/FPS/position",
    "/FPS/x",
    "/FPS/y",
    "/HEIGHT_LIMIT/enabled",
    "/HURT_CAM/enabled",
    "/HURT_CAM/options/disableHurtCam",
    "/HYPIXEL_BEDWARS/HYPIXEL_BEDWARS_HEIGHT_LIMIT_CHILD/position",
    "/HYPIXEL_BEDWARS/HYPIXEL_BEDWARS_HEIGHT_LIMIT_CHILD/y",
    "/HYPIXEL_BEDWARS/HYPIXEL_BEDWARS_RESOURCE_COUNTER_CHILD/position",
    "/HYPIXEL_BEDWARS/HYPIXEL_BEDWARS_RESOURCE_COUNTER_CHILD/x",
    "/HYPIXEL_MOD/HYPIXEL_TPS/enabled",
    "/HYPIXEL_MOD/HYPIXEL_TPS/options/background",
    "/HYPIXEL_MOD/HYPIXEL_TPS/position",
    "/HYPIXEL_MOD/HYPIXEL_TPS/x",
    "/HYPIXEL_MOD/HYPIXEL_TPS/y",
    "/HYPIXEL_MOD/options/levelHead",
    "/INVENTORY_MOD/SLOT_BINDING/options/slotBindingKeybind",
    "/INVENTORY_MOD/SLOT_BINDING/options/slotBindingKeybindAlt",
    "/INVENTORY_MOD/SLOT_BINDING/options/slotBindingKeybindControl",
    "/INVENTORY_MOD/SLOT_BINDING/options/slotBindingKeybindShift",
    "/INVENTORY_MOD/SLOT_BINDING/options/slotBindingLockBound",
    "/INVENTORY_MOD/enabled",
    "/INVENTORY_MOD/options/dontResetCursorInventory",
    "/ITEM_COUNTER/position",
    "/ITEM_COUNTER/x",
    "/ITEM_COUNTER/y",
    "/ITEM_TRACKER/options/skyblockOnly",
    "/ITEM_TRACKER/options/textShadow",
    "/ITEM_TRACKER/position",
    "/ITEM_TRACKER/x",
    "/ITEM_TRACKER/y",
    "/KEYSTROKES/KEYSTROKE_KEY_A/position",
    "/KEYSTROKES/KEYSTROKE_KEY_A/x",
    "/KEYSTROKES/KEYSTROKE_KEY_A/y",
    "/KEYSTROKES/KEYSTROKE_KEY_D/position",
    "/KEYSTROKES/KEYSTROKE_KEY_D/x",
    "/KEYSTROKES/KEYSTROKE_KEY_D/y",
    "/KEYSTROKES/KEYSTROKE_KEY_MOUSE1/position",
    "/KEYSTROKES/KEYSTROKE_KEY_MOUSE1/x",
    "/KEYSTROKES/KEYSTROKE_KEY_MOUSE1/y",
    "/KEYSTROKES/KEYSTROKE_KEY_MOUSE2/position",
    "/KEYSTROKES/KEYSTROKE_KEY_MOUSE2/x",
    "/KEYSTROKES/KEYSTROKE_KEY_MOUSE2/y",
    "/KEYSTROKES/KEYSTROKE_KEY_S/position",
    "/KEYSTROKES/KEYSTROKE_KEY_S/x",
    "/KEYSTROKES/KEYSTROKE_KEY_S/y",
    "/KEYSTROKES/KEYSTROKE_KEY_SPACE/position",
    "/KEYSTROKES/KEYSTROKE_KEY_SPACE/x",
    "/KEYSTROKES/KEYSTROKE_KEY_SPACE/y",
    "/KEYSTROKES/KEYSTROKE_KEY_W/position",
    "/KEYSTROKES/KEYSTROKE_KEY_W/x",
    "/KEYSTROKES/KEYSTROKE_KEY_W/y",
    "/KEYSTROKES/enabled",
    "/KEYSTROKES/options/backgroundColor/chroma",
    "/KEYSTROKES/options/backgroundColor/value",
    "/KEYSTROKES/options/backgroundPressedColor/value",
    "/KEYSTROKES/options/textShadow",
    "/KEYSTROKES/position",
    "/KEYSTROKES/x",
    "/KEYSTROKES/y",
    "/LIGHT_OVERLAY/enabledToggle",
    "/MEMORY/options/backgroundColor/chroma",
    "/MEMORY/options/backgroundColor/value",
    "/MEMORY/position",
    "/MEMORY/x",
    "/MEMORY/y",
    "/MOMENTUM/position",
    "/MOMENTUM/x",
    "/MOMENTUM/y",
    "/MUMBLE_LINK/enabled",
    "/ONE_SEVEN_VISUALS/ONE_SEVEN_ANIMATIONS_LEGACY/enabled",
    "/ONE_SEVEN_VISUALS/ONE_SEVEN_ITEMS_LEGACY/enabled",
    "/ONE_SEVEN_VISUALS/enabled",
    "/PARTICLE_CHANGER/PARTICLE_CHANGER_BLOCK_CHILD/enabled",
    "/PARTICLE_CHANGER/PARTICLE_CHANGER_BLOCK_CHILD/options/hideParticle",
    "/PARTICLE_CHANGER/PARTICLE_CHANGER_EXPLOSION_CHILD/enabled",
    "/PARTICLE_CHANGER/PARTICLE_CHANGER_EXPLOSION_CHILD/options/hideParticle",
    "/PARTICLE_CHANGER/PARTICLE_CHANGER_EXPLOSION_CHILD/options/scale",
    "/PARTICLE_CHANGER/enabled",
    "/PING/PING_HUD/options/background",
    "/PING/PING_HUD/position",
    "/PING/PING_HUD/x",
    "/PING/PING_HUD/y",
    "/PING/enabled",
    "/PING/options/backgroundColor/chroma",
    "/PING/options/backgroundColor/value",
    "/PING/x",
    "/PING/y",
    "/PLAYTIME/position",
    "/PLAYTIME/x",
    "/PLAYTIME/y",
    "/POTION_EFFECTS/options/background",
    "/POTION_EFFECTS/options/excludePerm",
    "/POTION_EFFECTS/position",
    "/POTION_EFFECTS/x",
    "/POTION_EFFECTS/y",
    "/PVP_INFO/PVP_INFO_HEALTH_CHILD/position",
    "/PVP_INFO/PVP_INFO_HEALTH_CHILD/x",
    "/PVP_INFO/PVP_INFO_HEALTH_CHILD/y",
    "/PVP_INFO/PVP_INFO_MELEE_CHILD/position",
    "/PVP_INFO/PVP_INFO_MELEE_CHILD/x",
    "/PVP_INFO/PVP_INFO_MELEE_CHILD/y",
    "/PVP_INFO/PVP_INFO_PROJECTILE_CHILD/position",
    "/PVP_INFO/PVP_INFO_PROJECTILE_CHILD/x",
    "/PVP_INFO/PVP_INFO_PROJECTILE_CHILD/y",
    "/QUICKPLAY/enabled",
    "/QUICKPLAY/options/quickplayUIKeybind",
    "/QUICKPLAY/options/quickplayUIKeybindAlt",
    "/QUICKPLAY/options/quickplayUIKeybindControl",
    "/QUICKPLAY/options/quickplayUIKeybindShift",
    "/RADIO/enabled",
    "/REACH_DISPLAY/options/backgroundColor/chroma",
    "/REACH_DISPLAY/options/backgroundColor/value",
    "/REACH_DISPLAY/position",
    "/REACH_DISPLAY/x",
    "/REACH_DISPLAY/y",
    "/REWIND/REWIND_RECORDING_INDICATOR_CHILD/position",
    "/REWIND/REWIND_RECORDING_INDICATOR_CHILD/x",
    "/REWIND/REWIND_RECORDING_INDICATOR_CHILD/y",
    "/SATURATION/SATURATION_HUD_CHILD/position",
    "/SATURATION/SATURATION_HUD_CHILD/x",
    "/SATURATION/SATURATION_HUD_CHILD/y",
    "/SATURATION/enabled",
    "/SBA/SBA_BAIT_LIST_CHILD/x",
    "/SBA/SBA_BAIT_LIST_CHILD/y",
    "/SBA/SBA_BIRCH_PARK_RAINMAKER_CHILD/enabled",
    "/SBA/SBA_BIRCH_PARK_RAINMAKER_CHILD/x",
    "/SBA/SBA_BIRCH_PARK_RAINMAKER_CHILD/y",
    "/SBA/SBA_BONE_DISPLAY_CHILD/x",
    "/SBA/SBA_BONE_DISPLAY_CHILD/y",
    "/SBA/SBA_DARK_AUCTION_TIMER_CHILD/enabled",
    "/SBA/SBA_DARK_AUCTION_TIMER_CHILD/x",
    "/SBA/SBA_DARK_AUCTION_TIMER_CHILD/y",
    "/SBA/SBA_DEFENSE_ICON_CHILD/enabled",
    "/SBA/SBA_DEFENSE_ICON_CHILD/position",
    "/SBA/SBA_DEFENSE_ICON_CHILD/x",
    "/SBA/SBA_DEFENSE_ICON_CHILD/y",
    "/SBA/SBA_DEFENSE_TEXT_CHILD/enabled",
    "/SBA/SBA_DEFENSE_TEXT_CHILD/position",
    "/SBA/SBA_DEFENSE_TEXT_CHILD/x",
    "/SBA/SBA_DEFENSE_TEXT_CHILD/y",
    "/SBA/SBA_ENDSTONE_PROTECTOR_CHILD/enabled",
    "/SBA/SBA_ENDSTONE_PROTECTOR_CHILD/x",
    "/SBA/SBA_ENDSTONE_PROTECTOR_CHILD/y",
    "/SBA/SBA_FEATURE_WARNING_CHILD/position",
    "/SBA/SBA_FEATURE_WARNING_CHILD/x",
    "/SBA/SBA_FEATURE_WARNING_CHILD/y",
    "/SBA/SBA_HEALTH_BAR_CHILD/enabled",
    "/SBA/SBA_HEALTH_BAR_CHILD/x",
    "/SBA/SBA_HEALTH_BAR_CHILD/y",
    "/SBA/SBA_HEALTH_TEXT_CHILD/enabled",
    "/SBA/SBA_HEALTH_TEXT_CHILD/x",
    "/SBA/SBA_HEALTH_TEXT_CHILD/y",
    "/SBA/SBA_MANA_BAR_CHILD/enabled",
    "/SBA/SBA_MANA_BAR_CHILD/x",
    "/SBA/SBA_MANA_BAR_CHILD/y",
    "/SBA/SBA_MANA_TEXT_CHILD/enabled",
    "/SBA/SBA_MANA_TEXT_CHILD/x",
    "/SBA/SBA_MANA_TEXT_CHILD/y",
    "/SBA/SBA_POWER_ORB_STATUS_CHILD/enabled",
    "/SBA/SBA_POWER_ORB_STATUS_CHILD/options/scale",
    "/SBA/SBA_POWER_ORB_STATUS_CHILD/x",
    "/SBA/SBA_POWER_ORB_STATUS_CHILD/y",
    "/SBA/SBA_SKILL_DISPLAY_CHILD/enabled",
    "/SBA/SBA_SKILL_DISPLAY_CHILD/options/textColor/value",
    "/SBA/SBA_SKILL_DISPLAY_CHILD/y",
    "/SBA/SBA_SPEED_TEXT_CHILD/x",
    "/SBA/SBA_SPEED_TEXT_CHILD/y",
    "/SBA/SBA_SUMMONING_EYE_COUNTER_CHILD/x",
    "/SBA/SBA_SUMMONING_EYE_COUNTER_CHILD/y",
    "/SBA/SBA_TICKER_CHARGES_DISPLAY_CHILD/enabled",
    "/SBA/SBA_TICKER_CHARGES_DISPLAY_CHILD/x",
    "/SBA/SBA_TICKER_CHARGES_DISPLAY_CHILD/y",
    "/SBA/options/avoidPlacingEnchantItems",
    "/SBA/options/backpackPreviewAh",
    "/SBA/options/hideHealthBar",
    "/SBA/options/hidePlayersNearNPC",
    "/SBA/options/ignoreItemFrameClicks",
    "/SBA/options/showBackpackHoldingShift",
    "/SBA/options/warpAdvancedMode",
    "/SBA/skyblockAddonsMagmaTimer/enabled",
    "/SBA/skyblockAddonsMagmaTimer/options/showOnOtherGames",
    "/SBA/skyblockAddonsMagmaTimer/position",
    "/SBA/skyblockAddonsMagmaTimer/x",
    "/SBA/skyblockAddonsMagmaTimer/y",
    "/SCOREBOARD/options/backgroundColor/chroma",
    "/SCOREBOARD/options/backgroundColor/value",
    "/SCROLLABLE_TOOLTIPS/enabled",
    "/SHINY_POTS/options/coloredPotions",
    "/SKYBLOCK/BETTERMAP_PRIMARY/options/primaryBetterMapCurrentRoomInfoScale",
    "/SKYBLOCK/BETTERMAP_PRIMARY/options/scale",
    "/SKYBLOCK/BETTERMAP_PRIMARY/position",
    "/SKYBLOCK/BETTERMAP_PRIMARY/x",
    "/SKYBLOCK/BETTERMAP_PRIMARY/y",
    "/SKYBLOCK/SKYBLOCK_BLAZE_SLAYER/enabled",
    "/SKYBLOCK/SKYBLOCK_BLAZE_SLAYER/position",
    "/SKYBLOCK/SKYBLOCK_BLAZE_SLAYER/x",
    "/SKYBLOCK/SKYBLOCK_BLAZE_SLAYER/y",
    "/SKYBLOCK/SKYBLOCK_CHOCOLATE_FACTORY/enabled",
    "/SKYBLOCK/SKYBLOCK_CROESUS_CHESTS/enabled",
    "/SKYBLOCK/SKYBLOCK_CRYSTAL_HOLLOWS_MAP/enabled",
    "/SKYBLOCK/SKYBLOCK_CRYSTAL_HOLLOWS_MAP/options/scale",
    "/SKYBLOCK/SKYBLOCK_CRYSTAL_HOLLOWS_MAP/position",
    "/SKYBLOCK/SKYBLOCK_CRYSTAL_HOLLOWS_MAP/x",
    "/SKYBLOCK/SKYBLOCK_CRYSTAL_HOLLOWS_MAP/y",
    "/SKYBLOCK/SKYBLOCK_DAMAGE_SPLASH/enabled",
    "/SKYBLOCK/SKYBLOCK_DEF_HUD/enabled",
    "/SKYBLOCK/SKYBLOCK_DEF_HUD/position",
    "/SKYBLOCK/SKYBLOCK_DEF_HUD/x",
    "/SKYBLOCK/SKYBLOCK_DEF_HUD/y",
    "/SKYBLOCK/SKYBLOCK_DRAGON_FEATURES/enabled",
    "/SKYBLOCK/SKYBLOCK_DRAGON_FEATURES/position",
    "/SKYBLOCK/SKYBLOCK_DRAGON_FEATURES/y",
    "/SKYBLOCK/SKYBLOCK_DUNGEON_BAT_HELPER/enabled",
    "/SKYBLOCK/SKYBLOCK_DUNGEON_BAT_HELPER/options/batHitbox",
    "/SKYBLOCK/SKYBLOCK_DUNGEON_BAT_HELPER/options/batHitboxColor/value",
    "/SKYBLOCK/SKYBLOCK_DUNGEON_BLOOD_CAMP_HELPER/enabled",
    "/SKYBLOCK/SKYBLOCK_DUNGEON_HIGHLIGHT_DOORS/enabled",
    "/SKYBLOCK/SKYBLOCK_DUNGEON_PUZZLES/enabled",
    "/SKYBLOCK/SKYBLOCK_DUNGEON_SCORE_ALERT/enabled",
    "/SKYBLOCK/SKYBLOCK_DUNGEON_TIMER/enabled",
    "/SKYBLOCK/SKYBLOCK_DUNGEON_TIMER/position",
    "/SKYBLOCK/SKYBLOCK_DUNGEON_TIMER/y",
    "/SKYBLOCK/SKYBLOCK_ENDERMAN_SLAYER/enabled",
    "/SKYBLOCK/SKYBLOCK_ENDERMAN_SLAYER/options/scale",
    "/SKYBLOCK/SKYBLOCK_ENDERMAN_SLAYER/position",
    "/SKYBLOCK/SKYBLOCK_ENDERMAN_SLAYER/x",
    "/SKYBLOCK/SKYBLOCK_ENDERMAN_SLAYER/y",
    "/SKYBLOCK/SKYBLOCK_EXPERIMENT_SOLVERS/enabled",
    "/SKYBLOCK/SKYBLOCK_FARMING_HUD/enabled",
    "/SKYBLOCK/SKYBLOCK_FARMING_HUD/position",
    "/SKYBLOCK/SKYBLOCK_FARMING_HUD/y",
    "/SKYBLOCK/SKYBLOCK_FISHING_MARKER/enabled",
    "/SKYBLOCK/SKYBLOCK_GARDEN_PESTS/enabled",
    "/SKYBLOCK/SKYBLOCK_GARDEN_PESTS/position",
    "/SKYBLOCK/SKYBLOCK_GARDEN_PESTS/y",
    "/SKYBLOCK/SKYBLOCK_GLACITE_COMMISSIONS/enabled",
    "/SKYBLOCK/SKYBLOCK_GLACITE_COMMISSIONS/options/skipPositionKeyBind",
    "/SKYBLOCK/SKYBLOCK_HEALTH_HUD/enabled",
    "/SKYBLOCK/SKYBLOCK_HEALTH_HUD/position",
    "/SKYBLOCK/SKYBLOCK_HEALTH_HUD/x",
    "/SKYBLOCK/SKYBLOCK_HEALTH_HUD/y",
    "/SKYBLOCK/SKYBLOCK_HIGHLIGHT_SPIRIT_BOW/enabled",
    "/SKYBLOCK/SKYBLOCK_HOPPITY_EGG_HUD/enabled",
    "/SKYBLOCK/SKYBLOCK_KUUDRA/enabled",
    "/SKYBLOCK/SKYBLOCK_KUUDRA/options/kuudraAlerts",
    "/SKYBLOCK/SKYBLOCK_KUUDRA_ARMOR_STACKS_HUD/enabled",
    "/SKYBLOCK/SKYBLOCK_KUUDRA_ARMOR_STACKS_HUD/position",
    "/SKYBLOCK/SKYBLOCK_KUUDRA_ARMOR_STACKS_HUD/x",
    "/SKYBLOCK/SKYBLOCK_KUUDRA_ARMOR_STACKS_HUD/y",
    "/SKYBLOCK/SKYBLOCK_LIVID_SOLVER_HUD/enabled",
    "/SKYBLOCK/SKYBLOCK_LOCK_MOUSE/options/lockMouseKeybind",
    "/SKYBLOCK/SKYBLOCK_LOCK_MOUSE/options/lockMouseKeybindAlt",
    "/SKYBLOCK/SKYBLOCK_LOCK_MOUSE/options/lockMouseKeybindControl",
    "/SKYBLOCK/SKYBLOCK_LOCK_MOUSE/options/lockMouseKeybindShift",
    "/SKYBLOCK/SKYBLOCK_LOCK_MOUSE/position",
    "/SKYBLOCK/SKYBLOCK_LOCK_MOUSE/x",
    "/SKYBLOCK/SKYBLOCK_LOCK_MOUSE/y",
    "/SKYBLOCK/SKYBLOCK_MANA_HUD/enabled",
    "/SKYBLOCK/SKYBLOCK_MANA_HUD/position",
    "/SKYBLOCK/SKYBLOCK_MANA_HUD/x",
    "/SKYBLOCK/SKYBLOCK_MANA_HUD/y",
    "/SKYBLOCK/SKYBLOCK_NETHER_BOSS_HUD/enabled",
    "/SKYBLOCK/SKYBLOCK_NETHER_BOSS_HUD/position",
    "/SKYBLOCK/SKYBLOCK_NETHER_BOSS_HUD/y",
    "/SKYBLOCK/SKYBLOCK_POWDER_TRACKER_HUD/enabled",
    "/SKYBLOCK/SKYBLOCK_SPEED_HUD/position",
    "/SKYBLOCK/SKYBLOCK_SPEED_HUD/x",
    "/SKYBLOCK/SKYBLOCK_SPEED_HUD/y",
    "/SKYBLOCK/SKYBLOCK_TERMINAL_SOLVERS/enabled",
    "/SKYBLOCK/SKYBLOCK_TERMINAL_SOLVERS/options/blockWrongTerminalClicks",
    "/SKYBLOCK/SKYBLOCK_VISITOR_HUD/enabled",
    "/SKYBLOCK/SKYBLOCK_VISITOR_HUD/position",
    "/SKYBLOCK/SKYBLOCK_VISITOR_HUD/x",
    "/SKYBLOCK/SKYBLOCK_VISITOR_HUD/y",
    "/SKYBLOCK/SKYBLOCK_VISITOR_TRACKER_HUD/enabled",
    "/SKYBLOCK/SKYBLOCK_VISITOR_TRACKER_HUD/options/skyblockShowBits",
    "/SKYBLOCK/SKYBLOCK_VISITOR_TRACKER_HUD/options/skyblockShowCopperDye",
    "/SKYBLOCK/SKYBLOCK_VISITOR_TRACKER_HUD/options/skyblockShowDedicationFour",
    "/SKYBLOCK/SKYBLOCK_VISITOR_TRACKER_HUD/options/skyblockShowFloweringBouquet",
    "/SKYBLOCK/SKYBLOCK_VISITOR_TRACKER_HUD/options/skyblockShowGreenBandana",
    "/SKYBLOCK/SKYBLOCK_VISITOR_TRACKER_HUD/options/skyblockShowMusicRune",
    "/SKYBLOCK/SKYBLOCK_VISITOR_TRACKER_HUD/options/skyblockShowOvergrownGrass",
    "/SKYBLOCK/SKYBLOCK_VISITOR_TRACKER_HUD/options/skyblockShowSpaceHelmet",
    "/SKYBLOCK/SKYBLOCK_VISITOR_TRACKER_HUD/position",
    "/SKYBLOCK/SKYBLOCK_VISITOR_TRACKER_HUD/x",
    "/SKYBLOCK/SKYBLOCK_WHISPER_TRACKER_HUD/enabled",
    "/SKYBLOCK/SPIRIT_LEAP_OVERLAY/enabled",
    "/SKYBLOCK/STARRED_MOB_HIGHLIGHT/enabled",
    "/SKYBLOCK/TIERS_AS_STACK_SIZE/enabled",
    "/SKYBLOCK/TIERS_AS_STACK_SIZE/options/itemStarsAsStack",
    "/SKYBLOCK/TIERS_AS_STACK_SIZE/options/minionLevelAsStack",
    "/SKYBLOCK/TIERS_AS_STACK_SIZE/options/potionLevelAsStack",
    "/SKYBLOCK/enabled",
    "/SKYBLOCK/options/fixLavaBobber",
    "/SKYBLOCK/options/griffinBurrowEstimates",
    "/SKYBLOCK/options/hideMidasStaff",
    "/SKYBLOCK/options/highlightEndNodes",
    "/SKYBLOCK/options/highlightGlowingMushrooms",
    "/SKYBLOCK/options/onlyMoversOnSkyblock",
    "/SKYBLOCK/options/showGiantHPAtFeet",
    "/SKYBLOCK/options/showKuudraHealth",
    "/SKYBLOCK/options/skyBlockFinishedCommissions",
    "/SKYBLOCK/options/skyBlockMetalDetector",
    "/SKYBLOCK/options/skyBlockMiddleClickItems",
    "/SKYBLOCK/options/skyBlockOpenCommandsUIAlt",
    "/SKYBLOCK/options/skyBlockOpenCommandsUIControl",
    "/SKYBLOCK/options/skyBlockOpenCommandsUIShift",
    "/SKYBLOCK/options/skyBlockWishingCompass",
    "/SKYBLOCK/options/skyblockFishingHidePlayers",
    "/SKYBLOCK/options/skyblockFishingHotspotLocator",
    "/SKYBLOCK/options/slayerBossTimer",
    "/SKYBLOCK/options/slayerMiniBossAlert",
    "/SKYBLOCK/options/vampireIchorDisplay",
    "/SKYBLOCK/options/vampireSteakDisplay",
    "/STOPWATCH/options/backgroundColor/chroma",
    "/STOPWATCH/options/backgroundColor/value",
    "/STOPWATCH/position",
    "/STOPWATCH/x",
    "/STOPWATCH/y",
    "/TIME_CHANGER/enabled",
    "/TIME_CHANGER/options/horizonYLevel",
    "/TIME_CHANGER/options/timeChangerTime",
    "/TOGGLE_SNEAK/TOGGLE_SNEAK_HUD_CHILD/options/iconMode",
    "/TOGGLE_SNEAK/TOGGLE_SNEAK_HUD_CHILD/position",
    "/TOGGLE_SNEAK/TOGGLE_SNEAK_HUD_CHILD/x",
    "/TOGGLE_SNEAK/TOGGLE_SNEAK_HUD_CHILD/y",
    "/TOGGLE_SNEAK/options/flyBoostAmount",
    "/TOGGLE_SNEAK/position",
    "/TOGGLE_SNEAK/y",
    "/TOTEM_COUNTER/TOTEM_COUNTER_HUD_CHILD/position",
    "/TOTEM_COUNTER/TOTEM_COUNTER_HUD_CHILD/x",
    "/TOTEM_COUNTER/TOTEM_COUNTER_HUD_CHILD/y",
    "/UHC_OVERLAY/enabled",
    "/UHC_OVERLAY/options/goldAppleScale",
    "/UHC_OVERLAY/options/goldIngotScale",
    "/UHC_OVERLAY/options/goldNuggetScale",
    "/UHC_OVERLAY/options/goldOreScale",
    "/UHC_OVERLAY/options/skullScale",
    "/WORLDEDIT_CUI/options/positionOneColor/value",
    "/WORLDEDIT_CUI/options/positionTwoColor/value",
    "/ZOOM/options/zoomKeybind",
    "/achievements",
    "/backgroundColor/chroma",
    "/backgroundColor/value",
    "/chatHeight",
    "/emoteWheelKeybind",
    "/mainMenuMuted",
    "/nametag",
    "/redString",
    "/showInF5",
    "/stackMessages",
    "/toggleChat",
    "/toggleChatAlt",
    "/toggleChatControl",
    "/toggleChatShift",
    "/transparentBackground",
    "/variableZoom",
    "accessibilityOnboarded",
    "advancedItemTooltips",
    "ao",
    "attackIndicator",
    "autoJump",
    "autoSuggestions",
    "backgroundForChatOnly",
    "biomeBlendRadius",
    "bobView",
    "chatBackgroundOpacity",
    "chatColors",
    "chatDelay",
    "chatHeightFocused",
    "chatHeightUnfocused",
    "chatLineSpacing",
    "chatLinks",
    "chatLinksPrompt",
    "chatOpacity",
    "chatScale",
    "chatVisibility",
    "chatWidth",
    "chunkBuilder",
    "clouds",
    "damageTiltStrength",
    "darknessEffectScale",
    "directionalAudio",
    "discrete_mouse_scroll",
    "enableVsync",
    "entityDistanceScaling",
    "entityShadows",
    "fancyGraphics",
    "fastRender",
    "fov",
    "fovEffectScale",
    "fullscreen",
    "fullscreenResolution",
    "gamma",
    "glDebugVerbosity",
    "glintSpeed",
    "glintStrength",
    "graphicsMode",
    "guiScale",
    "heldItemTooltips",
    "hideLightningFlashes",
    "hideMatchedNames",
    "hideServerAddress",
    "highContrast",
    "incompatibleResourcePacks",
    "invertYMouse",
    "key_Freelook",
    "key_key.advancements",
    "key_key.attack",
    "key_key.back",
    "key_key.chat",
    "key_key.command",
    "key_key.drop",
    "key_key.forward",
    "key_key.fullscreen",
    "key_key.hotbar.1",
    "key_key.hotbar.2",
    "key_key.hotbar.3",
    "key_key.hotbar.4",
    "key_key.hotbar.5",
    "key_key.hotbar.6",
    "key_key.hotbar.7",
    "key_key.hotbar.8",
    "key_key.hotbar.9",
    "key_key.inventory",
    "key_key.jump",
    "key_key.left",
    "key_key.loadToolbarActivator",
    "key_key.lunarclient.freelook",
    "key_key.lunarclient.menu",
    "key_key.lunarclient.toggleSprint",
    "key_key.lunarclient.waypoint",
    "key_key.lunarclient.zoom",
    "key_key.pickItem",
    "key_key.playerlist",
    "key_key.right",
    "key_key.saveToolbarActivator",
    "key_key.screenshot",
    "key_key.smoothCamera",
    "key_key.sneak",
    "key_key.socialInteractions",
    "key_key.spectatorOutlines",
    "key_key.sprint",
    "key_key.swapOffhand",
    "key_key.togglePerspective",
    "key_key.use",
    "key_of.key.zoom",
    "lang",
    "mainHand",
    "maxFps",
    "menuBackgroundBlurriness",
    "mipmapLevels",
    "modelPart_cape",
    "modelPart_hat",
    "modelPart_jacket",
    "modelPart_left_pants_leg",
    "modelPart_left_sleeve",
    "modelPart_right_pants_leg",
    "modelPart_right_sleeve",
    "monochromeLogo",
    "mouseSensitivity",
    "mouseWheelSensitivity",
    "narrator",
    "notificationDisplayTime",
    "onlyShowSecureChat",
    "operatorItemsTab",
    "overrideHeight",
    "overrideWidth",
    "panoramaScrollSpeed",
    "particles",
    "pauseOnLostFocus",
    "prioritizeChunkUpdates",
    "rawMouseInput",
    "realmsNotifications",
    "reducedDebugInfo",
    "renderClouds",
    "renderDistance",
    "resourcePacks",
    "screenEffectScale",
    "showAutosaveIndicator",
    "showSubtitles",
    "simulationDistance",
    "skipMultiplayerWarning",
    "skipRealms32bitWarning",
    "soundCategory_ambient",
    "soundCategory_block",
    "soundCategory_hostile",
    "soundCategory_master",
    "soundCategory_music",
    "soundCategory_neutral",
    "soundCategory_player",
    "soundCategory_record",
    "soundCategory_ui",
    "soundCategory_voice",
    "soundCategory_weather",
    "syncChunkWrites",
    "textBackgroundOpacity",
    "toggleCrouch",
    "toggleSprint",
    "touchscreen",
    "tutorialStep",
    "useNativeTransport",
    "useVbo",
];
const MAX_VALUE_BYTES: usize = 64 * 1024;
const MAX_VALUE_DEPTH: usize = 16;
const CHECKSUM_BYTES: usize = 32;
const INVALID_CODE: &str = "This share code appears to be damaged or unsupported.";
const UNSAFE_SETTINGS: &str = "Some selected settings cannot be shared safely.";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShareMetadata {
    pub minecraft_version: Option<String>,
    pub lunar_version: Option<String>,
    pub platform: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hud_viewport: Option<HudViewport>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShareEnvelope {
    pub format_version: u32,
    pub created_at: String,
    pub application_version: String,
    pub metadata: ShareMetadata,
    pub settings: Vec<Setting>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodingResult {
    pub code: String,
    pub compressed_bytes: usize,
    pub uncompressed_bytes: usize,
    pub setting_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireEnvelope {
    format_version: u32,
    created_at: String,
    application_version: String,
    metadata: WireMetadata,
    #[serde(deserialize_with = "deserialize_list")]
    settings: Vec<WireSetting>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireMetadata {
    minecraft_version: Option<String>,
    lunar_version: Option<String>,
    platform: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireSetting {
    source: String,
    file_kind: String,
    profile: String,
    pointer: String,
    #[serde(deserialize_with = "deserialize_scalar")]
    value: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompactEnvelope {
    format_version: u32,
    timestamp: i64,
    application_version: String,
    minecraft_version: Option<String>,
    lunar_version: Option<String>,
    platform: u8,
    #[serde(deserialize_with = "deserialize_groups")]
    groups: Vec<CompactGroup>,
}

#[derive(Clone, Debug)]
struct CompactHudEnvelope {
    compact: CompactEnvelope,
    hud_viewport: Option<HudViewport>,
}

impl Serialize for CompactHudEnvelope {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(8))?;
        sequence.serialize_element(&self.compact.format_version)?;
        sequence.serialize_element(&self.compact.timestamp)?;
        sequence.serialize_element(&self.compact.application_version)?;
        sequence.serialize_element(&self.compact.minecraft_version)?;
        sequence.serialize_element(&self.compact.lunar_version)?;
        sequence.serialize_element(&self.compact.platform)?;
        sequence.serialize_element(&self.compact.groups)?;
        sequence.serialize_element(
            &self
                .hud_viewport
                .as_ref()
                .map(|viewport| (viewport.width, viewport.height)),
        )?;
        sequence.end()
    }
}

impl<'de> Deserialize<'de> for CompactHudEnvelope {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Groups(Vec<CompactGroup>);
        impl<'de> Deserialize<'de> for Groups {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                deserialize_groups(deserializer).map(Self)
            }
        }
        struct HudVisitor;
        impl<'de> Visitor<'de> for HudVisitor {
            type Value = CompactHudEnvelope;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an eight-field share envelope")
            }

            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let format_version = sequence
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let timestamp = sequence
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let application_version = sequence
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                let minecraft_version = sequence
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(3, &self))?;
                let lunar_version = sequence
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let platform = sequence
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(5, &self))?;
                let Groups(groups) = sequence
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(6, &self))?;
                let viewport: Option<(f64, f64)> = sequence
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(7, &self))?;
                if sequence.next_element::<de::IgnoredAny>()?.is_some() {
                    return Err(de::Error::invalid_length(9, &self));
                }
                Ok(CompactHudEnvelope {
                    compact: CompactEnvelope {
                        format_version,
                        timestamp,
                        application_version,
                        minecraft_version,
                        lunar_version,
                        platform,
                        groups,
                    },
                    hud_viewport: viewport.map(|(width, height)| HudViewport { width, height }),
                })
            }
        }
        deserializer.deserialize_tuple(8, HudVisitor)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompactGroup {
    kind: u8,
    profile: u16,
    #[serde(deserialize_with = "deserialize_list")]
    settings: Vec<CompactSetting>,
}

#[derive(Clone, Debug)]
struct CompactSetting {
    dictionary: Option<u16>,
    prefix: u16,
    pointer: String,
    value: Value,
}

impl Serialize for CompactSetting {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence =
            serializer.serialize_seq(Some(if self.dictionary.is_some() || self.prefix == 0 {
                2
            } else {
                3
            }))?;
        if let Some(index) = self.dictionary {
            sequence.serialize_element(&(-1 - i32::from(index)))?;
        } else {
            if self.prefix != 0 {
                sequence.serialize_element(&self.prefix)?;
            }
            sequence.serialize_element(&self.pointer)?;
        }
        sequence.serialize_element(&self.value)?;
        sequence.end()
    }
}

impl<'de> Deserialize<'de> for CompactSetting {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        enum PointerStart {
            Full(String),
            Prefix(u16),
        }
        impl<'de> Deserialize<'de> for PointerStart {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct PointerVisitor;
                impl Visitor<'_> for PointerVisitor {
                    type Value = PointerStart;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a bounded pointer or prefix length")
                    }

                    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                        if value.len() > 512 {
                            return Err(E::custom("pointer limit exceeded"));
                        }
                        Ok(PointerStart::Full(value.to_string()))
                    }

                    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                        if value >= 0 {
                            return self.visit_u64(value as u64);
                        }
                        let index = value
                            .checked_neg()
                            .and_then(|index| index.checked_sub(1))
                            .and_then(|index| usize::try_from(index).ok())
                            .ok_or_else(|| E::custom("invalid dictionary index"))?;
                        let pointer = POINTER_DICTIONARY
                            .get(index)
                            .ok_or_else(|| E::custom("unknown dictionary index"))?;
                        Ok(PointerStart::Full((*pointer).to_string()))
                    }

                    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                        if !(1..=512).contains(&value) {
                            return Err(E::custom("invalid pointer prefix"));
                        }
                        Ok(PointerStart::Prefix(value as u16))
                    }
                }
                deserializer.deserialize_any(PointerVisitor)
            }
        }
        struct Scalar(Value);
        impl<'de> Deserialize<'de> for Scalar {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                deserialize_scalar(deserializer).map(Self)
            }
        }
        struct SettingVisitor;
        impl<'de> Visitor<'de> for SettingVisitor {
            type Value = CompactSetting;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a compact scalar setting")
            }

            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let start = sequence
                    .next_element::<PointerStart>()?
                    .ok_or_else(|| de::Error::custom("missing pointer"))?;
                let (prefix, pointer) = match start {
                    PointerStart::Full(pointer) => (0, pointer),
                    PointerStart::Prefix(prefix) => {
                        let pointer = sequence
                            .next_element::<String>()?
                            .ok_or_else(|| de::Error::custom("missing pointer suffix"))?;
                        if pointer.len() > 512 {
                            return Err(de::Error::custom("pointer limit exceeded"));
                        }
                        (prefix, pointer)
                    }
                };
                let value = sequence
                    .next_element::<Scalar>()?
                    .ok_or_else(|| de::Error::custom("missing setting value"))?
                    .0;
                if sequence.next_element::<de::IgnoredAny>()?.is_some() {
                    return Err(de::Error::custom("unexpected setting field"));
                }
                Ok(CompactSetting {
                    dictionary: None,
                    prefix,
                    pointer,
                    value,
                })
            }
        }
        deserializer.deserialize_seq(SettingVisitor)
    }
}

fn deserialize_list<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Vec<T>, D::Error> {
    struct SettingsVisitor<T>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>> Visitor<'de> for SettingsVisitor<T> {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a bounded array of settings")
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
            if sequence
                .size_hint()
                .is_some_and(|length| length > MAX_SETTINGS)
            {
                return Err(de::Error::custom("setting count limit exceeded"));
            }
            let mut settings =
                Vec::with_capacity(sequence.size_hint().unwrap_or(0).min(MAX_SETTINGS));
            while let Some(setting) = sequence.next_element::<T>()? {
                if settings.len() >= MAX_SETTINGS {
                    return Err(de::Error::custom("setting count limit exceeded"));
                }
                settings.push(setting);
            }
            Ok(settings)
        }
    }
    deserializer.deserialize_seq(SettingsVisitor(PhantomData))
}

fn deserialize_groups<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<CompactGroup>, D::Error> {
    struct GroupsVisitor;
    impl<'de> Visitor<'de> for GroupsVisitor {
        type Value = Vec<CompactGroup>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("bounded setting groups")
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
            if sequence
                .size_hint()
                .is_some_and(|length| length > MAX_SETTINGS)
            {
                return Err(de::Error::custom("group count limit exceeded"));
            }
            let mut groups =
                Vec::with_capacity(sequence.size_hint().unwrap_or(0).min(MAX_SETTINGS));
            let mut count = 0;
            while let Some(group) = sequence.next_element::<CompactGroup>()? {
                count += group.settings.len();
                if group.settings.is_empty() || count > MAX_SETTINGS || groups.len() >= MAX_SETTINGS
                {
                    return Err(de::Error::custom("setting count limit exceeded"));
                }
                groups.push(group);
            }
            Ok(groups)
        }
    }
    deserializer.deserialize_seq(GroupsVisitor)
}

fn deserialize_scalar<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Value, D::Error> {
    struct ScalarVisitor;
    impl<'de> Visitor<'de> for ScalarVisitor {
        type Value = Value;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a bounded scalar setting value")
        }

        fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
            Ok(Value::Bool(value))
        }

        fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
            Ok(Value::Number(value.into()))
        }

        fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
            Ok(Value::Number(value.into()))
        }

        fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
            serde_json::Number::from_f64(value)
                .map(Value::Number)
                .ok_or_else(|| E::custom("non-finite setting value"))
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
            if value.len() > (MAX_VALUE_BYTES - 2) / 6 {
                return Err(E::custom("setting value limit exceeded"));
            }
            Ok(Value::String(value.to_string()))
        }

        fn visit_string<E: de::Error>(self, value: String) -> Result<Value, E> {
            if value.len() > (MAX_VALUE_BYTES - 2) / 6 {
                return Err(E::custom("setting value limit exceeded"));
            }
            Ok(Value::String(value))
        }
    }
    deserializer.deserialize_any(ScalarVisitor)
}

impl From<&ShareEnvelope> for WireEnvelope {
    fn from(envelope: &ShareEnvelope) -> Self {
        Self {
            format_version: envelope.format_version,
            created_at: envelope.created_at.clone(),
            application_version: envelope.application_version.clone(),
            metadata: WireMetadata {
                minecraft_version: envelope.metadata.minecraft_version.clone(),
                lunar_version: envelope.metadata.lunar_version.clone(),
                platform: envelope.metadata.platform.clone(),
            },
            settings: envelope
                .settings
                .iter()
                .map(|setting| WireSetting {
                    source: setting.source.clone(),
                    file_kind: setting.file_kind.clone(),
                    profile: setting.profile.clone(),
                    pointer: setting.pointer.clone(),
                    value: setting.value.clone(),
                })
                .collect(),
        }
    }
}

impl From<WireEnvelope> for ShareEnvelope {
    fn from(envelope: WireEnvelope) -> Self {
        Self {
            format_version: envelope.format_version,
            created_at: envelope.created_at,
            application_version: envelope.application_version,
            metadata: ShareMetadata {
                minecraft_version: envelope.metadata.minecraft_version,
                lunar_version: envelope.metadata.lunar_version,
                platform: envelope.metadata.platform,
                hud_viewport: None,
            },
            settings: envelope
                .settings
                .into_iter()
                .map(|setting| Setting {
                    id: String::new(),
                    label: String::new(),
                    source: setting.source,
                    category: String::new(),
                    group: String::new(),
                    file_kind: setting.file_kind,
                    profile: setting.profile,
                    pointer: setting.pointer,
                    value: setting.value,
                })
                .collect(),
        }
    }
}

impl TryFrom<&ShareEnvelope> for CompactEnvelope {
    type Error = String;

    fn try_from(envelope: &ShareEnvelope) -> Result<Self, Self::Error> {
        let timestamp = DateTime::parse_from_rfc3339(&envelope.created_at)
            .map_err(|_| "These settings could not be encoded.".to_string())?
            .timestamp();
        let platform = match envelope.metadata.platform.as_str() {
            "windows" => 0,
            "macos" => 1,
            "linux" => 2,
            _ => return Err(UNSAFE_SETTINGS.to_string()),
        };
        let mut groups = BTreeMap::<(u8, u16), Vec<CompactSetting>>::new();
        for setting in &envelope.settings {
            let kind = kind_code(&setting.source, &setting.file_kind)
                .ok_or_else(|| UNSAFE_SETTINGS.to_string())?;
            if !valid_anonymous_profile(&setting.profile) {
                return Err(UNSAFE_SETTINGS.to_string());
            }
            let profile = setting
                .profile
                .strip_prefix("profile-")
                .unwrap()
                .parse::<u16>()
                .map_err(|_| UNSAFE_SETTINGS.to_string())?;
            groups
                .entry((kind, profile))
                .or_default()
                .push(CompactSetting {
                    dictionary: None,
                    prefix: 0,
                    pointer: setting.pointer.clone(),
                    value: setting.value.clone(),
                });
        }
        let groups = groups
            .into_iter()
            .map(|((kind, profile), mut settings)| {
                settings.sort_by(|left, right| left.pointer.cmp(&right.pointer));
                CompactGroup {
                    kind,
                    profile,
                    settings,
                }
            })
            .collect();
        Ok(Self {
            format_version: envelope.format_version,
            timestamp,
            application_version: envelope.application_version.clone(),
            minecraft_version: envelope.metadata.minecraft_version.clone(),
            lunar_version: envelope.metadata.lunar_version.clone(),
            platform,
            groups,
        })
    }
}

impl TryFrom<CompactEnvelope> for ShareEnvelope {
    type Error = ();

    fn try_from(envelope: CompactEnvelope) -> Result<Self, Self::Error> {
        let created_at = DateTime::from_timestamp(envelope.timestamp, 0)
            .ok_or(())?
            .to_rfc3339_opts(SecondsFormat::Secs, true);
        let platform = match envelope.platform {
            0 => "windows",
            1 => "macos",
            2 => "linux",
            _ => return Err(()),
        }
        .to_string();
        let mut settings = Vec::new();
        let mut groups = BTreeSet::new();
        for group in envelope.groups {
            if group.profile == 0
                || group.profile as usize > MAX_SETTINGS
                || !groups.insert((group.kind, group.profile))
            {
                return Err(());
            }
            let (source, file_kind) = decode_kind(group.kind).ok_or(())?;
            let profile = format!("profile-{}", group.profile);
            let mut previous = String::new();
            for setting in group.settings {
                let prefix = setting.prefix as usize;
                if prefix > previous.len() || !previous.is_char_boundary(prefix) {
                    return Err(());
                }
                let pointer = if prefix == 0 {
                    setting.pointer
                } else {
                    format!("{}{}", &previous[..prefix], setting.pointer)
                };
                if pointer.len() > 512 {
                    return Err(());
                }
                previous = pointer.clone();
                settings.push(Setting {
                    id: String::new(),
                    label: String::new(),
                    source: source.to_string(),
                    category: String::new(),
                    group: String::new(),
                    file_kind: file_kind.to_string(),
                    profile: profile.clone(),
                    pointer,
                    value: setting.value,
                });
            }
        }
        Ok(Self {
            format_version: envelope.format_version,
            created_at,
            application_version: envelope.application_version,
            metadata: ShareMetadata {
                minecraft_version: envelope.minecraft_version,
                lunar_version: envelope.lunar_version,
                platform,
                hud_viewport: None,
            },
            settings,
        })
    }
}

fn kind_code(source: &str, file_kind: &str) -> Option<u8> {
    match (source, file_kind) {
        ("minecraft", "options") => Some(0),
        ("lunar", "mods") => Some(1),
        ("lunar", "general") => Some(2),
        ("lunar", "controls") => Some(3),
        ("lunar", "performance") => Some(4),
        _ => None,
    }
}

fn decode_kind(kind: u8) -> Option<(&'static str, &'static str)> {
    match kind {
        0 => Some(("minecraft", "options")),
        1 => Some(("lunar", "mods")),
        2 => Some(("lunar", "general")),
        3 => Some(("lunar", "controls")),
        4 => Some(("lunar", "performance")),
        _ => None,
    }
}

pub fn encode(
    settings: Vec<Setting>,
    mut metadata: ShareMetadata,
) -> Result<EncodingResult, String> {
    if metadata.platform.len() > 16 {
        return Err(UNSAFE_SETTINGS.to_string());
    }
    metadata.platform = metadata.platform.to_ascii_lowercase();
    metadata.minecraft_version = metadata
        .minecraft_version
        .as_deref()
        .map(normalize_game_version)
        .transpose()
        .map_err(|_| UNSAFE_SETTINGS.to_string())?;
    metadata.lunar_version = metadata
        .lunar_version
        .as_deref()
        .map(normalize_game_version)
        .transpose()
        .map_err(|_| UNSAFE_SETTINGS.to_string())?;
    validate_metadata(&metadata).map_err(|_| UNSAFE_SETTINGS.to_string())?;
    let settings = prepare_settings(settings)?;
    validate_hud_profiles(&settings, &metadata)?;
    let envelope = ShareEnvelope {
        format_version: if metadata.hud_viewport.is_some() {
            3
        } else {
            2
        },
        created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        application_version: env!("CARGO_PKG_VERSION").to_string(),
        metadata,
        settings,
    };
    encode_envelope(&envelope)
}

pub fn decode(code: &str) -> Result<ShareEnvelope, String> {
    decode_checked(code).map_err(|_| INVALID_CODE.to_string())
}

fn prepare_settings(settings: Vec<Setting>) -> Result<Vec<Setting>, String> {
    if settings.is_empty() || settings.len() > MAX_SETTINGS {
        return Err("Select between 1 and 10,000 settings to create a share code.".to_string());
    }
    for setting in &settings {
        validate_record_size(setting).map_err(|_| UNSAFE_SETTINGS.to_string())?;
    }
    let profiles: BTreeSet<(String, String)> = settings
        .iter()
        .map(|setting| (setting.source.clone(), setting.profile.clone()))
        .collect();
    let mut counters = BTreeMap::<String, usize>::new();
    let profile_map: BTreeMap<(String, String), String> = profiles
        .into_iter()
        .map(|identity| {
            let counter = counters.entry(identity.0.clone()).or_default();
            *counter += 1;
            (identity, format!("profile-{counter}"))
        })
        .collect();
    let mut identities = BTreeSet::new();
    let mut prepared = Vec::with_capacity(settings.len());
    let mut budget = 0;
    for mut setting in settings {
        setting.profile = profile_map
            .get(&(setting.source.clone(), setting.profile.clone()))
            .ok_or_else(|| UNSAFE_SETTINGS.to_string())?
            .clone();
        let mut canonical =
            canonicalize_setting(&setting).map_err(|_| UNSAFE_SETTINGS.to_string())?;
        canonical.id = anonymous_id(&canonical);
        if !identities.insert(canonical.id.clone()) {
            return Err("The selection contains a duplicate setting.".to_string());
        }
        budget += record_budget(&canonical);
        if budget > MAX_UNCOMPRESSED_BYTES {
            return Err("This selection is too large for a share code.".to_string());
        }
        prepared.push(canonical);
    }
    prepared.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(prepared)
}

fn encode_envelope(envelope: &ShareEnvelope) -> Result<EncodingResult, String> {
    let compact = CompactEnvelope::try_from(envelope)?;
    let serialize = |compact: &CompactEnvelope| {
        if envelope.format_version == 2 {
            serialize_compact(compact)
        } else {
            serialize_compact(&CompactHudEnvelope {
                compact: compact.clone(),
                hud_viewport: envelope.metadata.hud_viewport.clone(),
            })
        }
    };
    let dictionary = serialize(&dictionary_pointers(compact.clone()))?;
    let plain = serialize(&compact)?;
    let prefixed = serialize(&prefix_pointers(compact))?;
    let (serialized, compressed) = [dictionary, plain, prefixed]
        .into_iter()
        .min_by_key(|(_, compressed)| compressed.len())
        .unwrap();
    let prefix = if envelope.format_version == 2 {
        COMPACT_PREFIX
    } else {
        PREFIX
    };
    let code = frame_payload(prefix, &compressed);
    if code.len() > MAX_CODE_BYTES {
        return Err("This selection is too large for a share code.".to_string());
    }
    Ok(EncodingResult {
        code,
        compressed_bytes: compressed.len(),
        uncompressed_bytes: serialized.len(),
        setting_count: envelope.settings.len(),
    })
}

fn serialize_compact<T: Serialize>(envelope: &T) -> Result<(Vec<u8>, Vec<u8>), String> {
    let serialized = rmp_serde::to_vec(envelope)
        .map_err(|_| "These settings could not be encoded.".to_string())?;
    if serialized.len() > MAX_UNCOMPRESSED_BYTES {
        return Err("This selection is too large for a share code.".to_string());
    }
    let compressed = compress_compact(&serialized)
        .map_err(|_| "These settings could not be compressed.".to_string())?;
    Ok((serialized, compressed))
}

fn dictionary_pointers(mut envelope: CompactEnvelope) -> CompactEnvelope {
    for group in &mut envelope.groups {
        for setting in &mut group.settings {
            if let Ok(index) = POINTER_DICTIONARY.binary_search(&setting.pointer.as_str()) {
                setting.dictionary = Some(index as u16);
            }
        }
    }
    envelope
}

fn prefix_pointers(mut envelope: CompactEnvelope) -> CompactEnvelope {
    for group in &mut envelope.groups {
        let mut previous = String::new();
        for setting in &mut group.settings {
            let full = setting.pointer.clone();
            let mut prefix = previous
                .bytes()
                .zip(full.bytes())
                .take_while(|(left, right)| left == right)
                .count();
            while !full.is_char_boundary(prefix) {
                prefix -= 1;
            }
            if prefix >= 3 {
                setting.prefix = prefix as u16;
                setting.pointer = full[prefix..].to_string();
            }
            previous = full;
        }
    }
    envelope
}

fn compress_compact(serialized: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut reader = brotli::CompressorReader::new(Cursor::new(serialized), 4096, 11, 22);
    let mut compressed = Vec::new();
    reader.read_to_end(&mut compressed)?;
    Ok(compressed)
}

fn frame_payload(prefix: &str, compressed: &[u8]) -> String {
    let mut framed = Vec::with_capacity(CHECKSUM_BYTES + compressed.len());
    framed.extend_from_slice(&Sha256::digest(compressed));
    framed.extend_from_slice(compressed);
    format!("{prefix}{}", URL_SAFE_NO_PAD.encode(framed))
}

fn decode_checked(code: &str) -> Result<ShareEnvelope, ()> {
    if code.len() > MAX_CODE_BYTES {
        return Err(());
    }
    let code = code.trim();
    let (version, encoded) = if let Some(encoded) = code.strip_prefix(PREFIX) {
        (3, encoded)
    } else if let Some(encoded) = code.strip_prefix(COMPACT_PREFIX) {
        (2, encoded)
    } else if let Some(encoded) = code.strip_prefix(LEGACY_PREFIX) {
        (1, encoded)
    } else {
        return Err(());
    };
    if encoded.is_empty()
        || !encoded
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(());
    }
    let framed = URL_SAFE_NO_PAD.decode(encoded).map_err(|_| ())?;
    if framed.len() <= CHECKSUM_BYTES {
        return Err(());
    }
    let (checksum, compressed) = framed.split_at(CHECKSUM_BYTES);
    if checksum != Sha256::digest(compressed).as_slice() {
        return Err(());
    }
    let mut serialized = Vec::new();
    if version == 1 {
        let mut decoder =
            zstd::stream::read::Decoder::new(Cursor::new(compressed)).map_err(|_| ())?;
        decoder.window_log_max(23).map_err(|_| ())?;
        decoder
            .take((MAX_UNCOMPRESSED_BYTES + 1) as u64)
            .read_to_end(&mut serialized)
            .map_err(|_| ())?;
    } else {
        serialized = decompress_compact(compressed)?;
    }
    if serialized.is_empty() || serialized.len() > MAX_UNCOMPRESSED_BYTES {
        return Err(());
    }
    let mut deserializer = rmp_serde::Deserializer::new(Cursor::new(&serialized));
    deserializer.set_max_depth(32);
    let envelope = if version == 1 {
        ShareEnvelope::from(WireEnvelope::deserialize(&mut deserializer).map_err(|_| ())?)
    } else if version == 2 {
        ShareEnvelope::try_from(CompactEnvelope::deserialize(&mut deserializer).map_err(|_| ())?)?
    } else {
        let envelope = CompactHudEnvelope::deserialize(&mut deserializer).map_err(|_| ())?;
        let mut decoded = ShareEnvelope::try_from(envelope.compact)?;
        decoded.metadata.hud_viewport = envelope.hud_viewport;
        decoded
    };
    if deserializer.position() != serialized.len() as u64 {
        return Err(());
    }
    validate_envelope(envelope, version)
}

fn decompress_compact(compressed: &[u8]) -> Result<Vec<u8>, ()> {
    if brotli_window_bits(*compressed.first().ok_or(())?).is_none_or(|bits| bits > 23) {
        return Err(());
    }
    let mut state = brotli::BrotliState::new(
        brotli::enc::StandardAlloc::default(),
        brotli::enc::StandardAlloc::default(),
        brotli::enc::StandardAlloc::default(),
    );
    let mut available_in = compressed.len();
    let mut input_offset = 0;
    let mut total_out = 0;
    let mut serialized = Vec::new();
    let mut output = [0_u8; 16 * 1024];
    loop {
        let mut available_out = output.len();
        let mut output_offset = 0;
        let result = brotli::BrotliDecompressStream(
            &mut available_in,
            &mut input_offset,
            compressed,
            &mut available_out,
            &mut output_offset,
            &mut output,
            &mut total_out,
            &mut state,
        );
        if serialized.len() + output_offset > MAX_UNCOMPRESSED_BYTES {
            return Err(());
        }
        serialized.extend_from_slice(&output[..output_offset]);
        match result {
            brotli::BrotliResult::ResultSuccess => {
                return if available_in == 0 && input_offset == compressed.len() {
                    Ok(serialized)
                } else {
                    Err(())
                };
            }
            brotli::BrotliResult::NeedsMoreOutput => {}
            _ => return Err(()),
        }
    }
}

fn brotli_window_bits(first: u8) -> Option<u8> {
    if first & 1 == 0 {
        return Some(16);
    }
    let bits = (first >> 1) & 7;
    if bits != 0 {
        return Some(17 + bits);
    }
    match (first >> 4) & 7 {
        0 => Some(17),
        1 => None,
        bits => Some(8 + bits),
    }
}

fn validate_envelope(mut envelope: ShareEnvelope, version: u32) -> Result<ShareEnvelope, ()> {
    if envelope.format_version != version
        || envelope.settings.is_empty()
        || envelope.settings.len() > MAX_SETTINGS
        || envelope.created_at.len() > 64
        || DateTime::parse_from_rfc3339(&envelope.created_at).is_err()
        || !valid_version(&envelope.application_version)
    {
        return Err(());
    }
    validate_metadata(&envelope.metadata)?;
    let mut identities = BTreeSet::new();
    for setting in &mut envelope.settings {
        validate_record_size(setting)?;
        if !valid_anonymous_profile(&setting.profile) {
            return Err(());
        }
        let canonical = canonicalize_setting(setting).map_err(|_| ())?;
        setting.id = anonymous_id(setting);
        if !identities.insert(setting.id.clone()) {
            return Err(());
        }
        setting.label = canonical.label;
        setting.category = canonical.category;
        setting.group = canonical.group;
    }
    envelope
        .settings
        .sort_by(|left, right| left.id.cmp(&right.id));
    validate_hud_profiles(&envelope.settings, &envelope.metadata).map_err(|_| ())?;
    Ok(envelope)
}

fn validate_metadata(metadata: &ShareMetadata) -> Result<(), ()> {
    if !matches!(metadata.platform.as_str(), "windows" | "macos" | "linux")
        || metadata
            .minecraft_version
            .as_ref()
            .is_some_and(|version| !valid_game_version(version))
        || metadata
            .lunar_version
            .as_ref()
            .is_some_and(|version| !valid_game_version(version))
    {
        return Err(());
    }
    if let Some(viewport) = &metadata.hud_viewport {
        viewport.validate().map_err(|_| ())?;
    }
    Ok(())
}

fn validate_hud_profiles(settings: &[Setting], metadata: &ShareMetadata) -> Result<(), String> {
    if metadata.hud_viewport.is_some()
        && settings
            .iter()
            .filter(|setting| crate::hud::is_adaptable_coordinate(setting))
            .map(|setting| setting.profile.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            > 1
    {
        return Err("Select one Lunar profile before sharing HUD positions.".into());
    }
    Ok(())
}

fn valid_version(version: &str) -> bool {
    !version.is_empty()
        && version.len() <= 64
        && version.as_bytes()[0].is_ascii_digit()
        && version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._+-".contains(&byte))
}

fn normalize_game_version(version: &str) -> Result<String, ()> {
    if version.len() > 64 {
        return Err(());
    }
    if valid_game_version(version) {
        return Ok(version.to_string());
    }
    let core = version.split('-').next().ok_or(())?;
    if valid_game_version(core) {
        return Ok(core.to_string());
    }
    Err(())
}

fn valid_game_version(version: &str) -> bool {
    if version.len() > 64 {
        return false;
    }
    let parts: Vec<&str> = version.split('.').collect();
    if (1..=4).contains(&parts.len())
        && parts.iter().all(|part| {
            !part.is_empty() && part.len() <= 4 && part.bytes().all(|byte| byte.is_ascii_digit())
        })
    {
        return true;
    }
    let bytes = version.as_bytes();
    bytes.len() == 6
        && bytes[0..2].iter().all(u8::is_ascii_digit)
        && bytes[2] == b'w'
        && bytes[3..5].iter().all(u8::is_ascii_digit)
        && bytes[5].is_ascii_lowercase()
}

fn valid_anonymous_profile(profile: &str) -> bool {
    profile
        .strip_prefix("profile-")
        .and_then(|number| number.parse::<usize>().ok())
        .is_some_and(|number| {
            number > 0 && number <= MAX_SETTINGS && profile == format!("profile-{number}")
        })
}

fn anonymous_id(setting: &Setting) -> String {
    let mut digest = Sha256::new();
    for component in [
        &setting.source,
        &setting.file_kind,
        &setting.profile,
        &setting.pointer,
    ] {
        digest.update((component.len() as u32).to_le_bytes());
        digest.update(component.as_bytes());
    }
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn validate_record_size(setting: &Setting) -> Result<(), ()> {
    if setting.id.len() > 2_048
        || setting.label.len() > 256
        || setting.source.len() > 32
        || setting.category.len() > 128
        || setting.group.len() > 256
        || setting.file_kind.len() > 64
        || setting.profile.len() > 1_024
        || setting.pointer.len() > 1_024
        || value_budget(&setting.value, 0)? > MAX_VALUE_BYTES
    {
        return Err(());
    }
    Ok(())
}

fn value_budget(value: &Value, depth: usize) -> Result<usize, ()> {
    if depth > MAX_VALUE_DEPTH {
        return Err(());
    }
    let budget = match value {
        Value::Null => 4,
        Value::Bool(_) => 5,
        Value::Number(_) => 32,
        Value::String(value) => value
            .len()
            .checked_mul(6)
            .and_then(|size| size.checked_add(2))
            .ok_or(())?,
        Value::Array(values) => {
            let mut budget: usize = 2;
            for value in values {
                budget = budget
                    .checked_add(value_budget(value, depth + 1)? + 1)
                    .ok_or(())?;
                if budget > MAX_VALUE_BYTES {
                    return Err(());
                }
            }
            budget
        }
        Value::Object(values) => {
            let mut budget: usize = 2;
            for (key, value) in values {
                let key_budget = key
                    .len()
                    .checked_mul(6)
                    .and_then(|size| size.checked_add(4))
                    .ok_or(())?;
                budget = budget
                    .checked_add(key_budget)
                    .and_then(|size| size.checked_add(value_budget(value, depth + 1).ok()?))
                    .ok_or(())?;
                if budget > MAX_VALUE_BYTES {
                    return Err(());
                }
            }
            budget
        }
    };
    Ok(budget)
}

fn record_budget(setting: &Setting) -> usize {
    setting.id.len()
        + setting.label.len()
        + setting.source.len()
        + setting.category.len()
        + setting.group.len()
        + setting.file_kind.len()
        + setting.profile.len()
        + setting.pointer.len()
        + value_budget(&setting.value, 0).unwrap_or(MAX_VALUE_BYTES)
        + 128
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> Setting {
        Setting {
            id: "minecraft:/Users/private-name/game:options:fov".to_string(),
            label: "Private user label".to_string(),
            source: "minecraft".to_string(),
            category: "Private category".to_string(),
            group: "Private group".to_string(),
            file_kind: "options".to_string(),
            profile: "Private player profile".to_string(),
            pointer: "fov".to_string(),
            value: json!("0.5"),
        }
    }

    fn metadata() -> ShareMetadata {
        ShareMetadata {
            minecraft_version: Some("1.21.4".to_string()),
            lunar_version: None,
            platform: "macos".to_string(),
            hud_viewport: None,
        }
    }

    fn envelope() -> ShareEnvelope {
        decode(&encode(vec![fixture()], metadata()).unwrap().code).unwrap()
    }

    fn legacy_code(envelope: &ShareEnvelope) -> String {
        let serialized = rmp_serde::to_vec(&WireEnvelope::from(envelope)).unwrap();
        let compressed = zstd::stream::encode_all(Cursor::new(serialized), 3).unwrap();
        frame_payload(LEGACY_PREFIX, &compressed)
    }

    fn compact_code(envelope: &CompactEnvelope) -> String {
        let serialized = rmp_serde::to_vec(envelope).unwrap();
        frame_payload(COMPACT_PREFIX, &compress_compact(&serialized).unwrap())
    }

    #[test]
    fn round_trip_preserves_safe_settings_and_removes_local_identity() {
        let result = encode(vec![fixture()], metadata()).unwrap();
        assert!(result.code.starts_with(COMPACT_PREFIX));
        assert_eq!(result.setting_count, 1);
        let decoded = decode(&result.code).unwrap();
        assert_eq!(decoded.settings[0].value, json!("0.5"));
        assert_eq!(decoded.settings[0].profile, "profile-1");
        let text = serde_json::to_string(&decoded).unwrap();
        assert!(!text.contains("Private"));
        assert!(!text.contains("private-name"));
        assert!(!text.contains("/Users/"));
    }

    #[test]
    fn malformed_checksum_and_invalid_base64_are_rejected() {
        let result = encode(vec![fixture()], metadata()).unwrap();
        let mut bytes = URL_SAFE_NO_PAD
            .decode(result.code.strip_prefix(COMPACT_PREFIX).unwrap())
            .unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        assert!(decode(&format!(
            "{COMPACT_PREFIX}{}",
            URL_SAFE_NO_PAD.encode(bytes)
        ))
        .is_err());
        assert!(decode("PRS1:%%%bad").is_err());
        assert!(decode("PRS1:").is_err());
        assert!(decode("PRS2:abcd").is_err());
        assert!(decode("unrecognized").is_err());
    }

    #[test]
    fn future_envelope_versions_and_trailing_data_are_rejected() {
        let mut envelope = envelope();
        envelope.format_version = 4;
        assert!(decode(&encode_envelope(&envelope).unwrap().code).is_err());
        envelope.format_version = 1;
        let mut serialized = rmp_serde::to_vec(&WireEnvelope::from(&envelope)).unwrap();
        serialized.extend_from_slice(b"unexpected");
        let compressed = zstd::stream::encode_all(Cursor::new(serialized), 3).unwrap();
        assert!(decode(&frame_payload(LEGACY_PREFIX, &compressed)).is_err());
    }

    #[test]
    fn unsafe_records_are_rejected_even_with_a_valid_checksum() {
        let mut setting = fixture();
        setting.pointer = "accessToken".to_string();
        setting.value = json!("secret-token");
        assert!(encode(vec![setting.clone()], metadata()).is_err());
        let mut envelope = envelope();
        setting.profile = "profile-1".to_string();
        setting.id = anonymous_id(&setting);
        envelope.settings = vec![setting];
        assert!(decode(&encode_envelope(&envelope).unwrap().code).is_err());
    }

    #[test]
    fn duplicate_empty_and_oversized_selections_are_rejected() {
        assert!(encode(Vec::new(), metadata()).is_err());
        assert!(encode(vec![fixture(), fixture()], metadata()).is_err());
        assert!(encode(vec![fixture(); MAX_SETTINGS + 1], metadata()).is_err());
        assert!(decode(&"X".repeat(MAX_CODE_BYTES + 1)).is_err());
        let mut setting = fixture();
        setting.value = json!("X".repeat(MAX_VALUE_BYTES));
        assert!(encode(vec![setting], metadata()).is_err());
    }

    #[test]
    fn decompression_bombs_are_rejected() {
        let compressed =
            zstd::stream::encode_all(Cursor::new(vec![0; MAX_UNCOMPRESSED_BYTES + 1]), 3).unwrap();
        assert!(decode(&frame_payload(LEGACY_PREFIX, &compressed)).is_err());
    }

    #[test]
    fn metadata_paths_and_noncanonical_profiles_are_rejected() {
        let mut metadata = metadata();
        metadata.minecraft_version = Some("/Users/private-name/game".to_string());
        assert!(encode(vec![fixture()], metadata).is_err());
        let mut envelope = envelope();
        envelope.settings[0].profile = "player-name".to_string();
        envelope.settings[0].id = anonymous_id(&envelope.settings[0]);
        assert!(encode_envelope(&envelope).is_err());
        envelope.format_version = 1;
        assert!(decode(&legacy_code(&envelope)).is_err());
    }

    #[test]
    fn version_directory_suffixes_are_not_shared() {
        let mut metadata = metadata();
        metadata.minecraft_version = Some("1.21.4-PrivateProfileName".to_string());
        metadata.platform = "macOS".to_string();
        let envelope = decode(&encode(vec![fixture()], metadata).unwrap().code).unwrap();
        assert_eq!(
            envelope.metadata.minecraft_version.as_deref(),
            Some("1.21.4")
        );
        assert_eq!(envelope.metadata.platform, "macos");
        let mut metadata = envelope.metadata;
        metadata.lunar_version = Some("123SecretToken".to_string());
        assert!(encode(vec![fixture()], metadata).is_err());
    }

    #[test]
    fn deep_values_are_rejected_before_serialization() {
        let mut value = json!(true);
        for _ in 0..MAX_VALUE_DEPTH + 1 {
            value = json!([value]);
        }
        let mut setting = fixture();
        setting.value = value;
        assert!(encode(vec![setting], metadata()).is_err());
    }

    #[test]
    fn incoming_arrays_and_excessive_record_counts_are_rejected() {
        let mut envelope = envelope();
        envelope.settings[0].value = json!([true, false]);
        assert!(decode(&encode_envelope(&envelope).unwrap().code).is_err());
        envelope.settings[0].value = json!("0.5");
        envelope.settings = vec![envelope.settings[0].clone(); MAX_SETTINGS + 1];
        assert!(decode(&encode_envelope(&envelope).unwrap().code).is_err());
    }

    #[test]
    fn legacy_codes_keep_decoding_with_the_same_setting_values() {
        let mut original = envelope();
        original.format_version = 1;
        original.created_at = "2026-09-30T00:00:00Z".to_string();
        original.application_version = "0.1.0".to_string();
        let golden = "PRS1:cSLB5gWkb6dPd0ISCWOSXgjN7dDo4iZpZXLNLvFNjA0otS_9AFiRAgCVAbQyMDI2LTA5LTMwVDAwOjAwOjAwWqUwLjEuMJOmMS4yMS40wKVtYWNvc5GVqW1pbmVjcmFmdKdvcHRpb25zqXByb2ZpbGUtMaNmb3ajMC41";
        assert_eq!(decode(golden).unwrap(), original);
        let decoded = decode(&legacy_code(&original)).unwrap();
        assert_eq!(decoded, original);
        let modern = decode(&encode(decoded.settings, decoded.metadata).unwrap().code).unwrap();
        assert_eq!(modern.format_version, 2);
        assert_eq!(modern.settings[0].value, json!("0.5"));
    }

    #[test]
    fn compact_groups_reject_unknown_ids_duplicates_and_invalid_profiles() {
        let original = CompactEnvelope::try_from(&envelope()).unwrap();
        let mut candidate = original.clone();
        candidate.platform = 3;
        assert!(decode(&compact_code(&candidate)).is_err());
        candidate = original.clone();
        candidate.groups[0].kind = 255;
        assert!(decode(&compact_code(&candidate)).is_err());
        for profile in [0, MAX_SETTINGS as u16 + 1] {
            candidate = original.clone();
            candidate.groups[0].profile = profile;
            assert!(decode(&compact_code(&candidate)).is_err());
        }
        candidate = original.clone();
        candidate.groups.push(candidate.groups[0].clone());
        assert!(decode(&compact_code(&candidate)).is_err());
        candidate = original.clone();
        candidate.groups[0].settings.clear();
        assert!(decode(&compact_code(&candidate)).is_err());
        candidate = original.clone();
        candidate.timestamp = i64::MAX;
        assert!(decode(&compact_code(&candidate)).is_err());
    }

    #[test]
    fn compact_format_preserves_numeric_strings_integer_colors_and_floats() {
        let mut minecraft = fixture();
        minecraft.value = json!("0.50");
        let mut lunar = fixture();
        lunar.source = "lunar".to_string();
        lunar.file_kind = "general".to_string();
        lunar.pointer = "/backgroundColor/value".to_string();
        lunar.value = json!(-16777216_i64);
        let mut coordinate = lunar.clone();
        coordinate.file_kind = "mods".to_string();
        coordinate.pointer = "/FPS/x".to_string();
        coordinate.value = json!(0.15000000000000002_f64);
        let mut numeric_string = lunar.clone();
        numeric_string.file_kind = "mods".to_string();
        numeric_string.pointer = "/CROSSHAIR/options/crosshairGap".to_string();
        numeric_string.value = json!("2.50");
        let selected = vec![minecraft, lunar, coordinate, numeric_string];
        let decoded = decode(&encode(selected.clone(), metadata()).unwrap().code).unwrap();
        for setting in &decoded.settings {
            let expected = selected
                .iter()
                .find(|item| item.pointer == setting.pointer && item.source == setting.source)
                .unwrap();
            assert_eq!(setting.value, expected.value);
        }
        assert!(decoded
            .settings
            .iter()
            .find(|setting| setting.pointer == "/backgroundColor/value")
            .unwrap()
            .value
            .as_i64()
            .is_some());
    }

    #[test]
    fn compact_total_count_is_bounded_across_individually_valid_groups() {
        let mut candidate = CompactEnvelope::try_from(&envelope()).unwrap();
        candidate.groups[0].settings = vec![candidate.groups[0].settings[0].clone(); MAX_SETTINGS];
        let mut second = candidate.groups[0].clone();
        second.profile = 2;
        second.settings.truncate(1);
        candidate.groups.push(second);
        assert!(decode(&compact_code(&candidate)).is_err());
    }

    #[test]
    fn brotli_window_bombs_and_trailing_compressed_data_are_rejected() {
        for (header, expected) in [
            (0, Some(16)),
            (1, Some(17)),
            (3, Some(18)),
            (13, Some(23)),
            (15, Some(24)),
            (0x11, None),
            (0x21, Some(10)),
            (0x71, Some(15)),
        ] {
            assert_eq!(brotli_window_bits(header), expected);
        }
        assert!(decode(&frame_payload(PREFIX, &[15])).is_err());
        assert!(decode(&frame_payload(PREFIX, &[0x11, 0x1e])).is_err());
        let mut compressed = Vec::new();
        brotli::CompressorReader::new(
            Cursor::new(vec![0; MAX_UNCOMPRESSED_BYTES + 1]),
            4096,
            1,
            22,
        )
        .read_to_end(&mut compressed)
        .unwrap();
        assert!(decode(&frame_payload(PREFIX, &compressed)).is_err());
        let serialized =
            rmp_serde::to_vec(&CompactEnvelope::try_from(&envelope()).unwrap()).unwrap();
        let mut compressed = compress_compact(&serialized).unwrap();
        compressed.extend_from_slice(b"trailing compressed data");
        assert!(decode(&frame_payload(PREFIX, &compressed)).is_err());
        let mut serialized = serialized;
        serialized.extend_from_slice(b"trailing uncompressed data");
        assert!(decode(&frame_payload(
            PREFIX,
            &compress_compact(&serialized).unwrap()
        ))
        .is_err());
    }

    #[test]
    fn pointer_prefixes_restore_values_and_reject_invalid_boundaries() {
        let settings = [
            ("/FPS/enabled", json!(true)),
            ("/FPS/x", json!(0.25)),
            ("/FPS/y", json!(-0.6)),
            ("/CPS/enabled", json!(false)),
        ]
        .into_iter()
        .map(|(pointer, value)| {
            let mut setting = fixture();
            setting.source = "lunar".to_string();
            setting.file_kind = "mods".to_string();
            setting.pointer = pointer.to_string();
            setting.value = value;
            setting
        })
        .collect::<Vec<_>>();
        let decoded = decode(&encode(settings, metadata()).unwrap().code).unwrap();
        let compact = CompactEnvelope::try_from(&decoded).unwrap();
        let prefixed = prefix_pointers(compact.clone());
        assert!(prefixed.groups[0]
            .settings
            .iter()
            .any(|setting| setting.prefix > 0));
        assert_eq!(decode(&compact_code(&prefixed)).unwrap(), decoded);
        let plain = serialize_compact(&compact).unwrap();
        let smaller = serialize_compact(&prefixed).unwrap();
        let encoded = encode_envelope(&decoded).unwrap();
        let dictionary = serialize_compact(&dictionary_pointers(compact)).unwrap();
        assert_eq!(
            encoded.compressed_bytes,
            plain.1.len().min(smaller.1.len()).min(dictionary.1.len())
        );
        let mut invalid = prefixed.clone();
        invalid.groups[0].settings[0].prefix = 1;
        assert!(decode(&compact_code(&invalid)).is_err());
        invalid = prefixed.clone();
        invalid.groups[0].settings[1].prefix = 512;
        assert!(decode(&compact_code(&invalid)).is_err());
        invalid = prefixed.clone();
        invalid.groups[0].settings[0].pointer = "/é/x".to_string();
        invalid.groups[0].settings[1].prefix = 2;
        assert!(ShareEnvelope::try_from(invalid).is_err());
    }

    #[test]
    fn version_two_dictionary_is_frozen_independently_of_the_scanner_schema() {
        assert_eq!(POINTER_DICTIONARY.len(), 611);
        let mut digest = Sha256::new();
        for pointer in POINTER_DICTIONARY {
            digest.update(pointer.as_bytes());
            digest.update(b"\n");
        }
        let fingerprint = digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(
            fingerprint,
            "099b6870a9e420ca9d116422548c26524845439e4fccc7eb916a25485bdd38a5"
        );
        assert_eq!(POINTER_DICTIONARY[0], "/3D_SKINS/enabled");
        assert_eq!(POINTER_DICTIONARY[497], "fov");
        assert_eq!(POINTER_DICTIONARY[108], "/FPS/x");
        assert_eq!(POINTER_DICTIONARY[POINTER_DICTIONARY.len() - 1], "useVbo");
    }

    #[test]
    fn dictionary_indices_rebuild_full_pointers_and_remain_allowlisted() {
        let compact = CompactEnvelope::try_from(&envelope()).unwrap();
        let mut dictionary = dictionary_pointers(compact);
        assert!(dictionary.groups[0].settings[0].dictionary.is_some());
        assert_eq!(
            decode(&compact_code(&dictionary)).unwrap().settings[0].pointer,
            "fov"
        );
        dictionary.groups[0].settings[0].dictionary = Some(POINTER_DICTIONARY.len() as u16);
        assert!(decode(&compact_code(&dictionary)).is_err());
        dictionary.groups[0].settings[0].dictionary =
            Some(POINTER_DICTIONARY.binary_search(&"/FPS/x").unwrap() as u16);
        assert!(decode(&compact_code(&dictionary)).is_err());
        dictionary.groups[0].settings[0].dictionary = Some(u16::MAX);
        assert!(decode(&compact_code(&dictionary)).is_err());
    }

    #[test]
    fn single_component_year_versions_roundtrip_without_local_profile_suffixes() {
        for version in ["26", "26.1", "1.21.4", "24w14a"] {
            let mut metadata = metadata();
            metadata.minecraft_version = Some(version.to_string());
            let decoded = decode(&encode(vec![fixture()], metadata).unwrap().code).unwrap();
            assert_eq!(decoded.metadata.minecraft_version.as_deref(), Some(version));
        }
        let mut metadata = metadata();
        metadata.minecraft_version = Some("26-PrivateProfileName".to_string());
        let decoded = decode(&encode(vec![fixture()], metadata).unwrap().code).unwrap();
        assert_eq!(decoded.metadata.minecraft_version.as_deref(), Some("26"));
        assert!(!serde_json::to_string(&decoded)
            .unwrap()
            .contains("PrivateProfileName"));
        for version in [
            "12345",
            "26/private-name",
            "26.1.PrivateProfileName",
            "26-PrivateProfileName",
        ] {
            let mut forged = decoded.clone();
            forged.metadata.minecraft_version = Some(version.to_string());
            assert!(decode(&encode_envelope(&forged).unwrap().code).is_err());
        }
    }

    #[test]
    fn version_three_viewport_roundtrips_without_exposing_window_or_settings_metadata() {
        let mut metadata = metadata();
        metadata.hud_viewport = Some(HudViewport {
            width: 1512.0,
            height: 945.5,
        });
        let result = encode(vec![fixture()], metadata.clone()).unwrap();
        assert!(result.code.starts_with(PREFIX));
        let decoded = decode(&result.code).unwrap();
        assert_eq!(decoded.format_version, 3);
        assert_eq!(decoded.metadata, metadata);
        assert_eq!(decoded.settings[0].value, json!("0.5"));
        let framed = URL_SAFE_NO_PAD
            .decode(result.code.strip_prefix(PREFIX).unwrap())
            .unwrap();
        let payload = decompress_compact(&framed[CHECKSUM_BYTES..]).unwrap();
        let fields: Value = rmp_serde::from_slice(&payload).unwrap();
        assert_eq!(fields.as_array().unwrap().len(), 8);
        assert_eq!(fields[7], json!([1512.0, 945.5]));
        assert!(!serde_json::to_string(&decoded).unwrap().contains("Private"));
        let old = decode(
            &encode(vec![fixture()], super::tests::metadata())
                .unwrap()
                .code,
        )
        .unwrap();
        assert_eq!(old.format_version, 2);
        assert!(old.metadata.hud_viewport.is_none());
    }

    #[test]
    fn version_three_requires_exact_field_count_and_bounded_viewport_pair() {
        let original = CompactHudEnvelope {
            compact: CompactEnvelope {
                format_version: 3,
                ..CompactEnvelope::try_from(&envelope()).unwrap()
            },
            hud_viewport: Some(HudViewport {
                width: 960.0,
                height: 540.0,
            }),
        };
        let fields = serde_json::to_value(&original).unwrap();
        let code = |fields: &Value| {
            frame_payload(
                PREFIX,
                &compress_compact(&rmp_serde::to_vec(fields).unwrap()).unwrap(),
            )
        };
        assert!(decode(&code(&fields)).is_ok());
        let mut missing = fields.clone();
        missing.as_array_mut().unwrap().pop();
        assert!(decode(&code(&missing)).is_err());
        let mut extra = fields.clone();
        extra.as_array_mut().unwrap().push(Value::Null);
        assert!(decode(&code(&extra)).is_err());
        for viewport in [
            json!([0.0, 540.0]),
            json!([960.0, 32769.0]),
            json!([960.0]),
            json!([960.0, 540.0, 1]),
            json!({"width":960,"height":540}),
        ] {
            let mut invalid = fields.clone();
            invalid[7] = viewport;
            assert!(decode(&code(&invalid)).is_err());
        }
        let mut without_viewport = fields.clone();
        without_viewport[7] = Value::Null;
        assert!(decode(&code(&without_viewport))
            .unwrap()
            .metadata
            .hud_viewport
            .is_none());
        assert!(decode(&frame_payload(
            COMPACT_PREFIX,
            &compress_compact(&rmp_serde::to_vec(&fields).unwrap()).unwrap()
        ))
        .is_err());
        let compact = rmp_serde::to_vec(&original.compact).unwrap();
        assert!(decode(&frame_payload(PREFIX, &compress_compact(&compact).unwrap())).is_err());
        for width in [f64::NAN, f64::INFINITY, 32769.0] {
            let invalid = CompactHudEnvelope {
                hud_viewport: Some(HudViewport {
                    width,
                    height: 540.0,
                }),
                ..original.clone()
            };
            assert!(decode(&frame_payload(
                PREFIX,
                &serialize_compact(&invalid).unwrap().1
            ))
            .is_err());
            let mut metadata = metadata();
            metadata.hud_viewport = invalid.hud_viewport;
            assert!(encode(vec![fixture()], metadata).is_err());
        }
    }

    #[test]
    fn one_geometry_cannot_describe_multiple_lunar_profiles() {
        let mut first = fixture();
        first.source = "lunar".into();
        first.file_kind = "mods".into();
        first.pointer = "/FPS/x".into();
        first.value = json!(10.0);
        let mut second = first.clone();
        second.profile = "Other profile".into();
        let selected = vec![first, second];
        let mut metadata = metadata();
        metadata.hud_viewport = Some(HudViewport {
            width: 960.0,
            height: 540.0,
        });
        assert_eq!(
            encode(selected.clone(), metadata.clone()).unwrap_err(),
            "Select one Lunar profile before sharing HUD positions."
        );
        let mut forged = envelope();
        forged.format_version = 3;
        forged.metadata = metadata;
        forged.settings = prepare_settings(selected).unwrap();
        assert!(decode(&encode_envelope(&forged).unwrap().code).is_err());
    }
}
