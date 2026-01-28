# `barnacle-lib`

## Graph Database

Barnacle uses a graph database called [agdb](https://agdb.agnesoft.com) to store its data. The schema is as follows:

```mermaid
graph TD
    Game[Game]
    Profile[Profile]
    Mod[Mod]
    ModEntry[ModEntry]

    Game --> Profile
    Game --> Mod

    Profile --> ModEntry
    ModEntry --> Mod
```

Here's an example of what your database could look like:

```mermaid
graph LR
    %% Column 1: Game
    subgraph G[ ]
        Skyrim[Game: Skyrim SE]
    end

    %% Column 2: Profiles
    subgraph P[ ]
        DefaultProfile[Profile: Default]
        MageProfile[Profile: Mage Run]
    end

    %% Column 3: ModEntries
    subgraph ME[ ]
        ME1[ModEntry: USSEP enabled]
        ME2[ModEntry: Mysticism enabled]
        ME3[ModEntry: Adamant disabled]

        ME4[ModEntry: USSEP enabled]
        ME5[ModEntry: Mysticism enabled]
    end

    %% Column 4: Mods
    subgraph M[ ]
        USSEP[Mod: USSEP]
        Mysticism[Mod: Mysticism]
        Adamant[Mod: Adamant]
    end

    %% Game → Profile
    Skyrim --> DefaultProfile
    Skyrim --> MageProfile

    %% Profile → ModEntry
    DefaultProfile --> ME1
    DefaultProfile --> ME2
    DefaultProfile --> ME3

    MageProfile --> ME4
    MageProfile --> ME5

    %% ModEntry → Mod
    ME1 --> USSEP
    ME2 --> Mysticism
    ME3 --> Adamant

    ME4 --> USSEP
    ME5 --> Mysticism
```
