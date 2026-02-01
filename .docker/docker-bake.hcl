# Docker Bake file for building all backend images with shared cache
# This ensures the Rust compilation (builder stage) only happens once per platform
#
# Usage:
#   Full multi-arch build (slow with QEMU):
#     docker buildx bake backend
#
#   Native arch-specific builds (fast, for parallel CI):
#     docker buildx bake backend-amd64  # on amd64 runner
#     docker buildx bake backend-arm64  # on arm64 runner
#
#   Then merge manifests with docker buildx imagetools create

variable "REGISTRY" {
  default = "ghcr.io"
}

variable "REPO_OWNER" {
  default = ""
}

variable "TAG" {
  default = "latest"
}

# Architecture suffix for parallel builds (empty for final tags, "-amd64" or "-arm64" for arch builds)
variable "ARCH_SUFFIX" {
  default = ""
}

group "default" {
  targets = ["r_data_core", "r_data_core_worker", "r_data_core_maintenance"]
}

group "backend" {
  targets = ["r_data_core", "r_data_core_worker", "r_data_core_maintenance"]
}

# Architecture-specific groups for parallel CI builds
group "backend-amd64" {
  targets = ["r_data_core_amd64", "r_data_core_worker_amd64", "r_data_core_maintenance_amd64"]
}

group "backend-arm64" {
  targets = ["r_data_core_arm64", "r_data_core_worker_arm64", "r_data_core_maintenance_arm64"]
}

# Shared target configuration
# Note: context is relative to the bake file location, dockerfile is relative to context
target "_common" {
  context    = ".."
  dockerfile = ".docker/app/Dockerfile"
  platforms  = ["linux/amd64", "linux/arm64"]
}

# Architecture-specific base targets
target "_common_amd64" {
  context    = ".."
  dockerfile = ".docker/app/Dockerfile"
  platforms  = ["linux/amd64"]
}

target "_common_arm64" {
  context    = ".."
  dockerfile = ".docker/app/Dockerfile"
  platforms  = ["linux/arm64"]
}

# =============================================================================
# Multi-arch targets (for local builds or single-job CI)
# =============================================================================
target "r_data_core" {
  inherits = ["_common"]
  target   = "r_data_core"
  tags     = ["${REGISTRY}/${REPO_OWNER}/r-data-core:${TAG}"]
  output   = ["type=image,push=true,annotation-index.org.opencontainers.image.description=r_data_core backend API service."]
}

target "r_data_core_worker" {
  inherits = ["_common"]
  target   = "r_data_core_worker"
  tags     = ["${REGISTRY}/${REPO_OWNER}/r-data-core-worker:${TAG}"]
  output   = ["type=image,push=true,annotation-index.org.opencontainers.image.description=r_data_core background worker service."]
}

target "r_data_core_maintenance" {
  inherits = ["_common"]
  target   = "r_data_core_maintenance"
  tags     = ["${REGISTRY}/${REPO_OWNER}/r-data-core-maintenance:${TAG}"]
  output   = ["type=image,push=true,annotation-index.org.opencontainers.image.description=r_data_core maintenance/ops image for administrative and one-off tasks."]
}

# =============================================================================
# AMD64 targets (for parallel CI on amd64 runners)
# =============================================================================
target "r_data_core_amd64" {
  inherits = ["_common_amd64"]
  target   = "r_data_core"
  tags     = ["${REGISTRY}/${REPO_OWNER}/r-data-core:${TAG}-amd64"]
  output   = ["type=image,push=true"]
}

target "r_data_core_worker_amd64" {
  inherits = ["_common_amd64"]
  target   = "r_data_core_worker"
  tags     = ["${REGISTRY}/${REPO_OWNER}/r-data-core-worker:${TAG}-amd64"]
  output   = ["type=image,push=true"]
}

target "r_data_core_maintenance_amd64" {
  inherits = ["_common_amd64"]
  target   = "r_data_core_maintenance"
  tags     = ["${REGISTRY}/${REPO_OWNER}/r-data-core-maintenance:${TAG}-amd64"]
  output   = ["type=image,push=true"]
}

# =============================================================================
# ARM64 targets (for parallel CI on arm64 runners)
# =============================================================================
target "r_data_core_arm64" {
  inherits = ["_common_arm64"]
  target   = "r_data_core"
  tags     = ["${REGISTRY}/${REPO_OWNER}/r-data-core:${TAG}-arm64"]
  output   = ["type=image,push=true"]
}

target "r_data_core_worker_arm64" {
  inherits = ["_common_arm64"]
  target   = "r_data_core_worker"
  tags     = ["${REGISTRY}/${REPO_OWNER}/r-data-core-worker:${TAG}-arm64"]
  output   = ["type=image,push=true"]
}

target "r_data_core_maintenance_arm64" {
  inherits = ["_common_arm64"]
  target   = "r_data_core_maintenance"
  tags     = ["${REGISTRY}/${REPO_OWNER}/r-data-core-maintenance:${TAG}-arm64"]
  output   = ["type=image,push=true"]
}
