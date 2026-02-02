# Entities
game = { $count ->
    [1] Game
   *[other] Games
}
profile = { $count ->
    [1] Profile
   *[other] Profiles
}
mod = { $count ->
    [1] Mod
   *[other] Mods
}
tool = { $count ->
    [one] Tool
   *[other] Tools
}

# Actions
activate = Activate
add = Add
create = Create
cancel = Cancel
new = New

# Fields
name = Name
path = Path
