//! Benchmarks of the interaction getters (D-250, D-041). Run with
//! `cargo bench -p djs-model --bench interaction`.
//!
//! A handler calls them a few times per interaction; they must not allocate. The payloads are
//! shaped like real ones: a subcommand in a group with five options and resolved data, a user
//! select with five users, a modal with five labelled inputs.

use std::sync::LazyLock;

use divan::{Bencher, black_box, counter::BytesCount};
use djs_model::{
    gateway::{GatewayEvent, GatewayEventDeserializer},
    id::marker::UserMarker,
    interaction::{CommandData, ComponentData, Interaction, ModalSubmitData},
};
use serde::de::DeserializeSeed;
use serde_json::{Value, json};

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn main() {
    divan::main();
}

fn user(id: u64) -> Value {
    json!({"id": id.to_string(), "username": format!("user{id}"), "discriminator": "0",
           "global_name": null, "avatar": null})
}

fn member() -> Value {
    json!({"roles": ["1100000000000000001"], "joined_at": "2024-05-01T12:00:00.000000+00:00",
           "flags": 0, "pending": false, "permissions": "8", "nick": null})
}

fn interaction(kind: u8, data: &Value) -> Interaction {
    serde_json::from_value(json!({
        "id": "1300000000000000001", "application_id": "1300000000000000002", "type": kind,
        "token": "not-a-real-token-only-for-benchmarking-0123456789", "version": 1,
        "app_permissions": "0", "guild_id": "1300000000000000003",
        "channel_id": "1300000000000000004", "data": data,
    }))
    .unwrap()
}

/// `/mod config set` with five options (a string, an integer, a boolean, a user and a role)
/// and their resolved user, member and role.
static COMMAND: LazyLock<Interaction> = LazyLock::new(|| {
    let target = 1_300_000_000_000_000_011_u64;
    interaction(
        2,
        &json!({
            "id": "1300000000000000010", "name": "mod", "type": 1,
            "options": [{"name": "config", "type": 2, "options": [{"name": "set", "type": 1,
                "options": [
                    {"name": "reason", "type": 3, "value": "spam in #general"},
                    {"name": "days", "type": 4, "value": 7},
                    {"name": "silent", "type": 5, "value": true},
                    {"name": "target", "type": 6, "value": target.to_string()},
                    {"name": "role", "type": 8, "value": "1100000000000000001"}
                ]}]}],
            "resolved": {
                "users": {target.to_string(): user(target)},
                "members": {target.to_string(): member()},
                "roles": {"1100000000000000001": {
                    "id": "1100000000000000001", "name": "Muted", "color": 0, "hoist": false,
                    "position": 1, "permissions": "0", "managed": false, "mentionable": false,
                    "flags": 0, "colors": {"primary_color": 0, "secondary_color": null,
                                           "tertiary_color": null}}}
            }
        }),
    )
});

fn command() -> &'static CommandData {
    COMMAND.command().unwrap()
}

/// A user select with five users, all members.
static SELECT: LazyLock<Interaction> = LazyLock::new(|| {
    let ids: Vec<u64> = (0..5).map(|i| 1_300_000_000_000_000_020 + i).collect();
    let users: serde_json::Map<String, Value> =
        ids.iter().map(|&id| (id.to_string(), user(id))).collect();
    let members: serde_json::Map<String, Value> =
        ids.iter().map(|&id| (id.to_string(), member())).collect();
    interaction(
        3,
        &json!({
            "custom_id": "pick", "component_type": 5,
            "values": ids.iter().map(u64::to_string).collect::<Vec<_>>(),
            "resolved": {"users": users, "members": members}
        }),
    )
});

fn select() -> &'static ComponentData {
    SELECT.component().unwrap()
}

/// Four text inputs and a user select, each in a label.
static MODAL: LazyLock<Interaction> = LazyLock::new(|| {
    let mut components: Vec<Value> = (0..4)
        .map(|i| {
            json!({"type": 18, "id": i * 2, "component": {
                "type": 4, "id": i * 2 + 1, "custom_id": format!("field{i}"),
                "value": format!("an answer of a few words {i}")}})
        })
        .collect();
    components.push(json!({"type": 18, "id": 8, "component": {
        "type": 5, "id": 9, "custom_id": "users", "values": ["1300000000000000020"]}}));
    interaction(
        5,
        &json!({
            "custom_id": "form", "components": components,
            "resolved": {"users": {"1300000000000000020": user(1_300_000_000_000_000_020)}}
        }),
    )
});

fn modal() -> &'static ModalSubmitData {
    MODAL.modal().unwrap()
}

mod command {
    use super::{Bencher, black_box, command};

    /// The last of five options of a subcommand in a group.
    #[divan::bench]
    fn get_string(bencher: Bencher<'_, '_>) {
        let data = command();
        bencher.bench_local(|| data.get_string(black_box("role")).is_some());
    }

    #[divan::bench]
    fn get_integer(bencher: Bencher<'_, '_>) {
        let data = command();
        bencher.bench_local(|| data.get_integer(black_box("days")));
    }

    /// A missing option: every leaf option is compared.
    #[divan::bench]
    fn get_missing(bencher: Bencher<'_, '_>) {
        let data = command();
        bencher.bench_local(|| data.get_string(black_box("missing")));
    }

    /// The option's ID, then the resolved user.
    #[divan::bench]
    fn get_user(bencher: Bencher<'_, '_>) {
        let data = command();
        bencher.bench_local(|| data.get_user(black_box("target")).map(|user| user.id));
    }

    /// The user and the member (two lookups).
    #[divan::bench]
    fn get_member(bencher: Bencher<'_, '_>) {
        let data = command();
        bencher.bench_local(|| data.get_member(black_box("target")).is_some());
    }

    #[divan::bench]
    fn get_role(bencher: Bencher<'_, '_>) {
        let data = command();
        bencher.bench_local(|| data.get_role(black_box("role")).map(|role| role.id));
    }

    /// The group and the subcommand: what the router reads for a route.
    #[divan::bench]
    fn subcommand_path(bencher: Bencher<'_, '_>) {
        let data = black_box(command());
        bencher.bench_local(|| (data.subcommand_group(), data.subcommand()));
    }
}

mod component {
    use super::{Bencher, UserMarker, black_box, select};

    /// Parsing the five selected IDs.
    #[divan::bench]
    fn ids_5(bencher: Bencher<'_, '_>) {
        let data = black_box(select());
        bencher.bench_local(|| data.ids::<UserMarker>().count());
    }

    /// The five selected users, resolved.
    #[divan::bench]
    fn users_5(bencher: Bencher<'_, '_>) {
        let data = black_box(select());
        bencher.bench_local(|| data.users().count());
    }

    /// The five selected users with their members.
    #[divan::bench]
    fn members_5(bencher: Bencher<'_, '_>) {
        let data = black_box(select());
        bencher.bench_local(|| data.members().count());
    }
}

mod modal {
    use super::{Bencher, black_box, modal};

    /// The first of five labelled fields.
    #[divan::bench]
    fn text_value_first(bencher: Bencher<'_, '_>) {
        let data = modal();
        bencher.bench_local(|| data.text_value(black_box("field0")).map(str::len));
    }

    /// The last text input, after three others.
    #[divan::bench]
    fn text_value_last(bencher: Bencher<'_, '_>) {
        let data = modal();
        bencher.bench_local(|| data.text_value(black_box("field3")).map(str::len));
    }

    /// The fifth field, a user select, resolved.
    #[divan::bench]
    fn selected_users(bencher: Bencher<'_, '_>) {
        let data = modal();
        bencher.bench_local(|| data.selected_users(black_box("users")).map(Iterator::count));
    }
}

/// The kind accessors of `Interaction` (D-249).
#[divan::bench]
fn accessor(bencher: Bencher<'_, '_>) {
    let interaction = black_box(&*COMMAND);
    bencher.bench_local(|| interaction.command().is_some());
}

/// The `INTERACTION_CREATE` of `tests/stress_interactions.rs` in `djs-rs` (≈ 1.9 KB): a
/// `/mod config set` with five options, the guild, channel and member, and resolved data.
static DISPATCH: LazyLock<String> = LazyLock::new(|| {
    let sequence = 2;
    format!(
        r#"{{"t":"INTERACTION_CREATE","s":{sequence},"op":0,"d":{{"id":"{}","application_id":"1","type":2,"token":"not-a-real-token-only-for-benchmarking-0123456789-aWQ6MTMwMDAwMDAwMDAwMDAwMDAwMQ","version":1,"app_permissions":"2248473465835073","locale":"pl","guild_locale":"en-US","guild_id":"1400000000000000002","guild":{{"id":"1400000000000000002","locale":"en-US","features":["COMMUNITY","NEWS"]}},"channel_id":"1200000000000000020","channel":{{"id":"1200000000000000020","type":0,"name":"general","guild_id":"1400000000000000002","nsfw":false,"flags":0,"permissions":"2248473465835073","position":3,"topic":"Talk here","last_message_id":"1500000000000000099","rate_limit_per_user":0,"parent_id":"1200000000000000001"}},"member":{{"user":{{"id":"1300000000000000003","username":"nelly","discriminator":"0","global_name":"Nelly","avatar":"a_1269e74af4df7417b13759eae50c83dc","public_flags":0,"clan":null,"avatar_decoration_data":null}},"roles":["1100000000000000001","1100000000000000002"],"joined_at":"2024-05-01T12:00:00.000000+00:00","premium_since":null,"deaf":false,"mute":false,"flags":0,"pending":false,"nick":null,"avatar":null,"banner":null,"communication_disabled_until":null,"permissions":"2248473465835073"}},"entitlements":[],"authorizing_integration_owners":{{"0":"1400000000000000002"}},"context":0,"attachment_size_limit":10485760,"data":{{"id":"1600000000000000001","name":"mod","type":1,"guild_id":"1400000000000000002","options":[{{"name":"config","type":2,"options":[{{"name":"set","type":1,"options":[{{"name":"reason","type":3,"value":"spam in general, number {sequence}"}},{{"name":"days","type":4,"value":7}},{{"name":"silent","type":5,"value":true}},{{"name":"target","type":6,"value":"1300000000000000011"}},{{"name":"role","type":8,"value":"1100000000000000001"}}]}}]}}],"resolved":{{"users":{{"1300000000000000011":{{"id":"1300000000000000011","username":"spammer","discriminator":"0","global_name":null,"avatar":null,"public_flags":0}}}},"members":{{"1300000000000000011":{{"roles":[],"joined_at":"2026-11-01T12:00:00.000000+00:00","premium_since":null,"flags":0,"pending":false,"nick":null,"avatar":null,"banner":null,"communication_disabled_until":null,"permissions":"104324673"}}}},"roles":{{"1100000000000000001":{{"id":"1100000000000000001","name":"Muted","color":0,"colors":{{"primary_color":0,"secondary_color":null,"tertiary_color":null}},"hoist":false,"icon":null,"unicode_emoji":null,"position":1,"permissions":"0","managed":false,"mentionable":false,"flags":0}}}}}}}}}}}}"#,
        1_700_000_000_000_000_000_u64 + sequence
    )
});

/// Decoding the whole dispatch, as the gateway does for every interaction.
#[divan::bench]
fn decode_dispatch(bencher: Bencher<'_, '_>) {
    let json = DISPATCH.as_str();
    bencher.counter(BytesCount::of_str(json)).bench(|| {
        let seed = GatewayEventDeserializer::from_json(black_box(json)).unwrap();
        let event: GatewayEvent = seed
            .deserialize(&mut serde_json::Deserializer::from_str(json))
            .unwrap();
        event
    });
}
