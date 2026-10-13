# Changelog

## 1.0.0 (2026-10-13)


### Features

* **accounts:** implement CP02 local accounts and workspace onboarding ([1471ec0](https://github.com/hbjydev/convex/commit/1471ec0d4ed208dee24ddffdad1f70f31edc5eac))
* **auth:** add OIDC login and account method linking ([#3](https://github.com/hbjydev/convex/issues/3)) ([62e1762](https://github.com/hbjydev/convex/commit/62e176237dddc5b60b960135365476b7d07937c8))
* establish application foundation and split CI workflows ([39a97b7](https://github.com/hbjydev/convex/commit/39a97b7e81f19a7f59e0b49573601e17ef7edde8))
* **issues:** add selection formatting toolbar ([#5](https://github.com/hbjydev/convex/issues/5)) ([6909b48](https://github.com/hbjydev/convex/commit/6909b48333e24ce0c5e511c5b8cac68bef5f800b))
* **issues:** complete CP04 issue workflow ([#4](https://github.com/hbjydev/convex/issues/4)) ([ffdc039](https://github.com/hbjydev/convex/commit/ffdc039f19b5d67e53be9d2712b0f9c93358ea40))
* **mail:** improve email templates ([#8](https://github.com/hbjydev/convex/issues/8)) ([b23f309](https://github.com/hbjydev/convex/commit/b23f309e8dc5406f6ee958670d8f3a20140573d3))
* **server:** embed frontend for bundled deployments ([b9d48e3](https://github.com/hbjydev/convex/commit/b9d48e35b69ba1d3aeac8d6ae831339fb0fd5c0f))
* **ui:** add Linear-inspired navigation and workspace slugs ([89f55f3](https://github.com/hbjydev/convex/commit/89f55f34528082b488ba61a609f4c8572d380e0a))
* **ui:** rebuild with shadcn and tsr ([a8b9894](https://github.com/hbjydev/convex/commit/a8b9894b93ba31d45e593f693e12e500e8317a7f))
* **workspaces:** streamline creation and add verified deletion ([#7](https://github.com/hbjydev/convex/issues/7)) ([19f2672](https://github.com/hbjydev/convex/commit/19f267237d45817c06bef9e0736c580410e12123))


### Bug Fixes

* **smtp:** ignore blank username and password in smtp ([7218103](https://github.com/hbjydev/convex/commit/7218103e6683893ea0af6f8abe52ef4757fefc3b))
* use oci repo for helm chart instead of http ([af6771f](https://github.com/hbjydev/convex/commit/af6771fba99f686c8dcd10337de90ec1529dc10e))


### Code Refactoring

* **backend:** split domain crates and type entity IDs ([#6](https://github.com/hbjydev/convex/issues/6)) ([b351d72](https://github.com/hbjydev/convex/commit/b351d72733c4d91f1090c90dccb1c1539d1dce89))


### Build System

* **docker:** resolve migrations-related build failures ([6f9c09e](https://github.com/hbjydev/convex/commit/6f9c09e5d8e631f34cef6245034d656ea621f1e1))
* **helm:** create helm chart for convex ([bf3142c](https://github.com/hbjydev/convex/commit/bf3142ca8876c5dd32463f1c74be16ca0288e572))
* **web:** use oxlint & oxfmt over biome ([73b4e69](https://github.com/hbjydev/convex/commit/73b4e6990b6f379a70383f9d4373e105a50ef828))


### Continuous Integration

* don't run CI on release-please PRs ([65d020e](https://github.com/hbjydev/convex/commit/65d020e29cc88a628e323b484bb165a2d20f58ce))
* **release-please:** update config to be simple over the entire repo ([66d0f49](https://github.com/hbjydev/convex/commit/66d0f49ce68eb9c378fd9a4ea081be04dc595719))
* **release:** release-please and main-release workflow ([57beef9](https://github.com/hbjydev/convex/commit/57beef9c03cffa9fbc74c2079e23d65ddb28c6c9))


### Miscellaneous Chores

* improve healthcheck page ([1deaaf8](https://github.com/hbjydev/convex/commit/1deaaf8059526b8787adf0ab94cfe0c277ccd182))
* **release-please:** pin to 0.0.0 to trigger pre-release behaviour ([ca9d116](https://github.com/hbjydev/convex/commit/ca9d116e036b847092796291fb52539aa381cca7))
