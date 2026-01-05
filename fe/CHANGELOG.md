# Changelog

## [0.2.0](https://github.com/BentBr/r_data_core/compare/fe-v0.1.0...fe-v0.2.0) (2026-01-04)

### ⚠ BREAKING CHANGES

* license key creation and check via cronjobs

### Features

* a lot of fixes + mostly done DSL with validation and
  tests ([04f0524](https://github.com/BentBr/r_data_core/commit/04f0524f3a14af61f2a25d97f34e11d1b3d042b8))
* added a no-access page in order to signal users can login but
  l… ([6fae703](https://github.com/BentBr/r_data_core/commit/6fae703ab66a14d2f3f1482937cc285670cd097a))
* added a no-access page in order to signal users can login but lack appropriate right to open
  pages ([316e40e](https://github.com/BentBr/r_data_core/commit/316e40e82b987105fe3b9db42c70e1a2b45dd598))
* Added dismisable hints for mobile usage (
  &lt;1200px) ([4fdec9e](https://github.com/BentBr/r_data_core/commit/4fdec9e87621e380ca68e22063bc2fedbc945573))
* Added dismisable hints for mobile usage (
  &lt;1200px) ([f7d7034](https://github.com/BentBr/r_data_core/commit/f7d7034ad4e8354ff8193409543699e972086a7e))
* Added new fromDef to allow
  webhooks ([9f60669](https://github.com/BentBr/r_data_core/commit/9f60669fa79c9481cae9234fb88e093fb4efe629))
* Added new fromDef to allow
  webhooks ([ecc8e0c](https://github.com/BentBr/r_data_core/commit/ecc8e0c73c9660afcc01fb814f96eb5e86227c66))
* adding admin permission for
  resources ([d92d4b6](https://github.com/BentBr/r_data_core/commit/d92d4b69f2e8d642d93f8373ae71aee2fffec8ed))
* Adding version report to FE + setting FE release
  independently ([b71020f](https://github.com/BentBr/r_data_core/commit/b71020fc642360bd7a1e97b92b9e09c865cfc7ad))
* Adding version report to FE + setting FE release
  independently ([89f1666](https://github.com/BentBr/r_data_core/commit/89f1666c932c41588c6238fc1cd2524fff442633))
* comprehensive DSL rework (be + fe). now with better transform
  a… ([a290d1f](https://github.com/BentBr/r_data_core/commit/a290d1ff905a8598778c1fef9e46a085af0b00f8))
* comprehensive DSL rework (be + fe). now with better transform and
  stepping ([e94c0c4](https://github.com/BentBr/r_data_core/commit/e94c0c416b94b0ca6eae677753c3137b14cc7fbf))
* Fixed fe to support json (be already did
  it) ([cd1d29b](https://github.com/BentBr/r_data_core/commit/cd1d29b91241eec5b1139051b77cb4ca5c6a0198))
* license key creation and check via
  cronjobs ([d3392b9](https://github.com/BentBr/r_data_core/commit/d3392b9042795463f6daf463fc124b8baddd8d0b))
* making sure adding/changing steps json is working as
  well ([72a0a65](https://github.com/BentBr/r_data_core/commit/72a0a65e330e47099a807e8747ae92474f378253))
* making sure adding/changing steps json is working as
  well ([122b37c](https://github.com/BentBr/r_data_core/commit/122b37c3975a9cd25717af02acc82dc5abba8f2c))
* making sure the license shows
  expiry ([2092742](https://github.com/BentBr/r_data_core/commit/20927429ca0b9c89576386ea47ac3b78e5e58543))
* New DSL transforms to find and create entities
  on-the-fly ([efccd3d](https://github.com/BentBr/r_data_core/commit/efccd3db82c7ce15fcd2f1eb513372f85372dd77))
* pushing uuid-creation into db (away from rust
  cod) ([8578f8b](https://github.com/BentBr/r_data_core/commit/8578f8bad53744e28e7a794a22efbfdc457d002d))
* showing a banner to every user if the default password has not been changed
  yet. ([a849df6](https://github.com/BentBr/r_data_core/commit/a849df69b2537cd36f282210ecdd45b79173cab0))

### Bug Fixes

* allowed null messages and meta objects from
  API ([419275a](https://github.com/BentBr/r_data_core/commit/419275a575a45d077dc2c75ee31e21c621f7092f))
* allowed null messages and meta objects from
  API ([ec6295e](https://github.com/BentBr/r_data_core/commit/ec6295eeb5d2bcd25feef731594e5108553e9073))
* changes to ci to have the FE released as
  well ([fe6d78f](https://github.com/BentBr/r_data_core/commit/fe6d78f6beca5b584422af8ff35f0f251b3076f4))
* changes to ci to have the FE released as
  well ([1810c57](https://github.com/BentBr/r_data_core/commit/1810c5704505e3de55a0411027ced4512ce9f6b8))
* **ci:** making sure our admin fe is working in
  ci ([dec7fdc](https://github.com/BentBr/r_data_core/commit/dec7fdcbbba3e99b09759339df147f6330007052))
* making sure all errors, validations etc. are shown properly.
  err… ([61e1507](https://github.com/BentBr/r_data_core/commit/61e15072ed20f5891e5ddf6d04efd48ccf8da6e3))
* making sure all errors, validations etc. are shown properly. error + response handling rework in
  FE ([4524fd5](https://github.com/BentBr/r_data_core/commit/4524fd51c9e8a5f3e4c60b1330a0dc812ddd882e))
* making sure create buttons only exist for users who are
  permitte… ([e991ca5](https://github.com/BentBr/r_data_core/commit/e991ca526d94ffbaa3fdd467cc576808841f5f1e))
* making sure create buttons only exist for users who are permitted. Added proper fallback
  info ([228e012](https://github.com/BentBr/r_data_core/commit/228e0128d81bbbc135c6edd1f2b3b4129f0fa414))
* Making sure create buttons only exist if users are allowed to do
  so ([b65679b](https://github.com/BentBr/r_data_core/commit/b65679b267b15569acdad5d1bb0c15007d05ac69))
* Making sure create buttons only exist if users are allowed to do
  so ([d2653b1](https://github.com/BentBr/r_data_core/commit/d2653b149a3ef68b7849dd527658e1c479a3fc8e))
* making sure to have the trigger as type and not format
  defined ([885a991](https://github.com/BentBr/r_data_core/commit/885a991521e3402acf0ff79b6ae2a9f79ead62df))
* removed logout
  redirect ([1d173fd](https://github.com/BentBr/r_data_core/commit/1d173fd2a9021ae348a5a7b21a32c3d383257901))
* removed logout
  redirect ([530a389](https://github.com/BentBr/r_data_core/commit/530a389773bddfd174abd2631bdf2d0042ba9e9a))
* removed vuetify old icons in tree
  view ([a56675c](https://github.com/BentBr/r_data_core/commit/a56675c97dc9ffc97168628eed91f6ea92b79cf9))
* updating role handling to have users and roles taken care of separately. updating repository to include warnings in
  clippy for tests ([112733f](https://github.com/BentBr/r_data_core/commit/112733fa4650c06b59d12e2eb86123764e99ff36))
