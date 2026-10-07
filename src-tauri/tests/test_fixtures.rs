pub const PLAYER_A_ID: i64 = 1000000000001;
pub const PLAYER_B_ID: i64 = 2000000000002;
pub const CABIN_NAME: &str = "Cabin_stone_1";

pub fn generate_test_save_xml() -> String {
    format!(r#"<?xml version="1.0" encoding="utf-8"?>
<SaveGame xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <player>
    <name>PlayerA</name>
    <UniqueMultiplayerID>{a_id}</UniqueMultiplayerID>
    <homeLocation>FarmHouse</homeLocation>
    <houseUpgradeLevel>2</houseUpgradeLevel>
    <farmName>Emerald</farmName>
    <isMale>true</isMale>
    <items>
      <Item xsi:type="Tool">
        <name>Iridium Pickaxe</name>
        <upgradeLevel>4</upgradeLevel>
      </Item>
      <Item xsi:type="Object">
        <name>Parsnip Seeds</name>
        <stack>50</stack>
        <quality>0</quality>
      </Item>
    </items>
    <experiencePoints>
      <int>1500</int>
      <int>800</int>
      <int>0</int>
      <int>0</int>
      <int>0</int>
    </experiencePoints>
    <professions>
      <int>0</int>
    </professions>
    <friendshipData>
      <item>
        <key><string>Abigail</string></key>
        <value><Friendship><Points>2500</Points></Friendship></value>
      </item>
    </friendshipData>
    <questLog>
      <Quest><name>Introductions</name><completed>true</completed></Quest>
    </questLog>
    <mailReceived>
      <string>spring_1_year_1</string>
    </mailReceived>
    <modData>
      <customData key="mod_test_flag">alpha_42</customData>
    </modData>
  </player>
  <locations>
    <GameLocation xsi:type="Farm">
      <name>Farm</name>
      <buildings>
        <Building>
          <buildingType>Stone Cabin</buildingType>
          <tileX>42</tileX>
          <tileY>18</tileY>
          <indoors xsi:type="Cabin">
            <uniqueName>{cabin_name}</uniqueName>
            <upgradeLevel>1</upgradeLevel>
            <farmhand>
              <name>PlayerB</name>
              <UniqueMultiplayerID>{b_id}</UniqueMultiplayerID>
              <homeLocation>{cabin_name}</homeLocation>
              <houseUpgradeLevel>1</houseUpgradeLevel>
              <farmName>Emerald</farmName>
              <isMale>false</isMale>
              <items>
                <Item xsi:type="Tool">
                  <name>Fiberglass Rod</name>
                  <upgradeLevel>2</upgradeLevel>
                </Item>
                <Item xsi:type="Object">
                  <name>Bait</name>
                  <stack>30</stack>
                  <quality>0</quality>
                </Item>
              </items>
              <experiencePoints>
                <int>0</int>
                <int>0</int>
                <int>2200</int>
                <int>0</int>
                <int>0</int>
              </experiencePoints>
              <professions>
                <int>4</int>
              </professions>
              <friendshipData>
                <item>
                  <key><string>Leah</string></key>
                  <value><Friendship><Points>2000</Points></Friendship></value>
                </item>
              </friendshipData>
              <questLog>
                <Quest><name>Fish Frenzy</name><completed>false</completed></Quest>
              </questLog>
              <mailReceived>
                <string>willy_invitation</string>
              </mailReceived>
              <modData>
                <customData key="mod_test_flag">beta_99</customData>
              </modData>
            </farmhand>
          </indoors>
        </Building>
      </buildings>
    </GameLocation>
    <GameLocation xsi:type="FarmHouse">
      <name>FarmHouse</name>
      <upgradeLevel>2</upgradeLevel>
    </GameLocation>
  </locations>
  <currentSeason>summer</currentSeason>
  <dayOfMonth>18</dayOfMonth>
  <year>2</year>
  <dailyLuck>0.05</dailyLuck>
  <farmerTeam>
    <useSeparateWallets>false</useSeparateWallets>
  </farmerTeam>
</SaveGame>"#,
        a_id = PLAYER_A_ID,
        b_id = PLAYER_B_ID,
        cabin_name = CABIN_NAME
    )
}

pub fn generate_test_save_game_info_xml() -> String {
    format!(r#"<?xml version="1.0" encoding="utf-8"?>
<Farmer xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <name>PlayerA</name>
  <UniqueMultiplayerID>{a_id}</UniqueMultiplayerID>
  <homeLocation>FarmHouse</homeLocation>
  <houseUpgradeLevel>2</houseUpgradeLevel>
  <farmName>Emerald</farmName>
  <isMale>true</isMale>
  <items>
    <Item xsi:type="Tool">
      <name>Iridium Pickaxe</name>
      <upgradeLevel>4</upgradeLevel>
    </Item>
    <Item xsi:type="Object">
      <name>Parsnip Seeds</name>
      <stack>50</stack>
      <quality>0</quality>
    </Item>
  </items>
</Farmer>"#,
        a_id = PLAYER_A_ID
    )
}
