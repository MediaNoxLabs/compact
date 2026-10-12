# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Authenticate the final LocalUnit leaf test evolution with fixed owner authority."""
from recording_admission_inventory_jubjub import canonical
from recording_admission_inventory_test_adaptations import restore_tests, TestCheckpoint

MANIFEST_SHA = '31e6083012c725608e1424ff2dc7fff3ca838172d0ab9a0fee4ec9d5b89ebd3c'
PRIOR_MANIFEST_SHA = 'e9e5cb3217bec452fa16072a9906f78bbc9f74f6f48eee3fde4ecaacd07c2732'
OWNERS = {
    'tools/compact-rust-backend/src/recorded/profile_selection/jubjub_migration_tests.rs': (
        '5e59d83f2ea7a1a17b1bf166e3a6e26d03ee63a5a07acbe632da7b349f602ae1',
        'e5e159dbe0967d615609e37aaa4ecdca4d9a1ebb27e2c38486ec1524af658248',
    ),
    'tools/compact-rust-backend/src/recorded/profile_selection/context_migration_tests/routing.rs': (
        'daef435b975f919af2897353e1a96a082d9edf7ee6e87a08d170bb194ef6a0e7',
        '091249f445dbee5c3bb5049c8106a4926543506bafde9ae6fe1d836b570bd2d9',
    ),
    'tools/compact-rust-backend/src/recorded/profile_selection/kernel_migration_tests/routing.rs': (
        '6252ae3775f2203d7731578620964ff5bcddafc98ab987c62af21f24f667b221',
        'd455f7cc267e4f1a7b343b7a7ca9377f8e71bf5d70e53ac6fe51495f32e73168',
    ),
    'tools/compact-rust-backend/src/recorded/profile_selection/zswap_migration_tests.rs': (
        '3b49e9404eef90f6df88ca210f85b01cb6a305a2799c7c9487cd1d703bc03deb',
        '3404ba58904d7a6f62a1e69a5b3824717767533b9466f2f30f2ad4b4caf89ee0',
    ),
    'tools/compact-rust-backend/src/recorded/profile_selection/receive_migration_tests/routing.rs': (
        '034c8e9c1baaf96953c369789934b511cf7d9f81fc8a4dc65682d1512f139db0',
        '72c168c181efc0718ea2aa4ac800b82791f1faadd960ea207d4e7556032bc919',
    ),
    'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/routing.rs': (
        '33ea2c276a6dede61be2fad0b1c5880959c8d421e77c17e23afe1f98e1cea411',
        '4f1b302d924a89486bb020612857106a03b519ef10bf992de803223336092b33',
    ),
    'tools/compact-rust-backend/src/recorded/profile_selection/terminal_migration_tests/routing.rs': (
        '8c210a9b887c48d00f907849e5b8ba7d463fe9ad48618067cb96761710dc6c7b',
        '7d1df2b021c2a62af8c82869c8e2997e5a39ef5d0369d57a398c687ee3178491',
    ),
    'tools/compact-rust-backend/src/recorded/profile_selection/funded_mint_migration_tests.rs': (
        '9b58f225f75fa40cfa2208a2166ef1a13c9b008149ed5d861ea1fc56e1b7675f',
        '6e511caff9e5ce977d9ca519aa644af18c98de25c43543c3e4124bdd3e1ac9a0',
    ),
    'tools/compact-rust-backend/src/recorded/profile_selection/reset_payout_migration_tests.rs': (
        '5314ffd2f76ee3a62afbf82d17433a5acd7aab9c9c36fcc7ff8be514d1244261',
        '62cfee742619d8594fef2b5e43d660933f51a1d863c3b5dd68c26f66aa5d46be',
    ),
    'tools/compact-rust-backend/src/recorded/profile_selection/shielded_payout_migration_tests.rs': (
        '3bed33ed5151c92f5ece51b43ce94ad6db667d99c2cb95deb13f58a3da101da9',
        '3df37e2778d80af5b81c9d08f55596e67be5658ff0481be8585bb2beb2fd6016',
    ),
    'tools/compact-rust-backend/src/recorded/profile_selection/guarded_deposit_migration_tests/routing.rs': (
        '77c5979abec69f6a2d0aca46a878322305141571cff178506d7d58e373a1dde0',
        '2c03cd2a9bfd5359c29272e23e9e264d0454a83b21a4d2a91bb6cbec13e75d4b',
    ),
    'tools/compact-rust-backend/src/recorded/profile_selection/membership_migration_tests.rs': (
        '309abb8c50130a6de24296ea24c7a50d39ddfc85854ef1461fc492c026705edc',
        '3e00686d9813b7b37a0ba9b8306af5bfc8267417e47aafd6d2c66bdc426fd696',
    ),
    'tools/compact-rust-backend/src/recorded/profile_selection/merge_migration_tests.rs': (
        '1e9254240b6c20601dc4b6edd0369624de006aab4b1e4333c9f331e517fc05c6',
        'c48666d35425d42860f83a06109a45a2977830924cccd79f1f63a1a802caa76d',
    ),
    'tools/compact-rust-backend/src/recorded/profile_selection/send_migration_tests.rs': (
        '5b99a056bf1b5cc05bc3b146725c6688b453146ca4a350119bb27f840cb8a929',
        '3be956f8755bdcab5bcd5ebeba9e506a2be7bd82b9f918cb22e0c20803d51232',
    ),
    'tools/compact-rust-backend/src/recorded/typed_plan/composition/tests.rs': (
        '0429407c3943591996f43c8b4e0b1f33a740cd8d750e3326025df4bf0f79b65f',
        'a0765208843e342d7eceb65b118a45a508d37fdd38568f31d4c8ae7b9733aa49',
    ),
}


def predecessor_sources(root, extension):
    return restore_tests(root, extension, MANIFEST_SHA, OWNERS, 'value586')


class _TestCheckpoint(TestCheckpoint):
    """Only fifteen reviewed test owners can be projected to the predecessor."""
    def __init__(self, root, sources):
        super().__init__(root, sources, OWNERS, 'value586')


def checkpoint(root, extension, negation):
    if negation is None or canonical(negation) != PRIOR_MANIFEST_SHA:
        raise ValueError('value586: immutable negation585 predecessor required')
    return _TestCheckpoint(root, predecessor_sources(root, extension))
