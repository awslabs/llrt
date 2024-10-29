// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    collections::HashMap,
    net::{Ipv4Addr, Ipv6Addr},
    sync::Arc,
};

use llrt_utils::result::ResultExt;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rquickjs::{Ctx, Object, Result};
use sysinfo::Networks;

static NETWORKS: Lazy<Arc<Mutex<Networks>>> =
    Lazy::new(|| Arc::new(Mutex::new(Networks::new_with_refreshed_list())));

pub fn get_network_interfaces(ctx: Ctx<'_>) -> Result<HashMap<String, Vec<Object>>> {
    let mut map: HashMap<String, Vec<Object>> = HashMap::new();
    let networks = NETWORKS.lock();

    for (interface_name, network_data) in networks.iter() {
        let mut ifs = Vec::new();

        for ip_network in network_data.ip_networks() {
            let addr = &ip_network.addr.to_string();
            let is_ipv4 = addr.contains(".");
            let (is_internal, scope_id) = if is_ipv4 {
                get_attribute_ipv4(&ctx, addr)?
            } else {
                get_attribute_ipv6(&ctx, addr)?
            };

            let obj = Object::new(ctx.clone())?;
            obj.set("address", addr)?;
            obj.set(
                "netmask",
                if is_ipv4 {
                    prefix_to_netmask_ipv4(ip_network.prefix)
                } else {
                    prefix_to_netmask_ipv6(ip_network.prefix)
                }
                .to_string(),
            )?;
            obj.set("family", if is_ipv4 { "IPv4" } else { "IPv6" })?;
            obj.set("mac", network_data.mac_address().to_string())?;
            obj.set("internal", is_internal)?;
            obj.set("cidr", [addr, "/", &ip_network.prefix.to_string()].concat())?;
            if !is_ipv4 {
                obj.set("scopeid", scope_id)?;
            }

            ifs.push(obj);
        }
        if !ifs.is_empty() {
            map.insert(interface_name.to_string(), ifs);
        }
    }
    Ok(map)
}

fn prefix_to_netmask_ipv4(prefix: u8) -> Box<str> {
    let mut prefix = prefix;

    if prefix > 32 {
        return Box::from("");
    }

    let mut mask = [0u8; 4];

    #[allow(clippy::needless_range_loop)]
    for i in 0..4 {
        if prefix >= 8 {
            mask[i] = 255;
            prefix -= 8;
        } else if prefix > 0 {
            mask[i] = 255 << (8 - prefix);
            break;
        }
    }
    Box::from(Ipv4Addr::new(mask[0], mask[1], mask[2], mask[3]).to_string())
}

fn prefix_to_netmask_ipv6(prefix: u8) -> Box<str> {
    let mut prefix = prefix;

    if prefix > 128 {
        return Box::from("");
    }

    let mut mask = [0u16; 8];

    #[allow(clippy::needless_range_loop)]
    for i in 0..8 {
        if prefix >= 16 {
            mask[i] = 0xFFFF;
            prefix -= 16;
        } else if prefix > 0 {
            mask[i] = 0xFFFF << (16 - prefix);
            break;
        }
    }
    Box::from(
        Ipv6Addr::new(
            mask[0], mask[1], mask[2], mask[3], mask[4], mask[5], mask[6], mask[7],
        )
        .to_string(),
    )
}

fn get_attribute_ipv4(ctx: &Ctx<'_>, addr: &str) -> Result<(bool, u8)> {
    let addr = addr.parse::<Ipv4Addr>().or_throw(ctx)?;
    let is_internal = addr.is_broadcast()
        || addr.is_documentation()
        || addr.is_link_local()
        || addr.is_loopback()
        || addr.is_multicast()
        || addr.is_unspecified();
    let scope_id = 0; // For IPv4, ScopeID is a dummy value.

    Ok((is_internal, scope_id))
}

fn get_attribute_ipv6(ctx: &Ctx<'_>, addr: &str) -> Result<(bool, u8)> {
    let addr = addr.parse::<Ipv6Addr>().or_throw(ctx)?;
    let is_internal = addr.is_loopback() || addr.is_multicast() || addr.is_unspecified();
    let scope_id = 0; // ScopeID is not supported at this time.

    Ok((is_internal, scope_id))
}
