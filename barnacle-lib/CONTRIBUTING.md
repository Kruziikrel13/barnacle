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
        ME1[ModEntry: USSEP]
        ME2[ModEntry: Mysticism]
        ME3[ModEntry: Adamant]

        ME4[ModEntry: USSEP]
        ME5[ModEntry: Adamant]
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
    MageProfile --> ME1
    MageProfile --> ME2
    MageProfile --> ME3

    DefaultProfile --> ME4
    DefaultProfile --> ME5

    %% ModEntry → Mod
    ME1 --> USSEP
    ME2 --> Mysticism
    ME3 --> Adamant

    ME4 --> USSEP
    ME5 --> Adamant
```
