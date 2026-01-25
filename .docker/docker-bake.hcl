# Docker Bake file for building all backend images with shared cache
# This ensures the Rust compilation (builder stage) only happens once per platform

variable "REGISTRY" {
  default = "ghcr.io"
}

variable "REPO_OWNER" {
  default = ""
}

variable "TAG" {
  default = "latest"
}

group "default" {
  targets = ["r_data_core", "r_data_core_worker", "r_data_core_maintenance"]
}

group "backend" {
  targets = ["r_data_core", "r_data_core_worker", "r_data_core_maintenance"]
}

# Shared target configuration
# Note: context is relative to the bake file location, dockerfile is relative to context
target "_common" {
  context    = ".."
  dockerfile = ".docker/app/Dockerfile"
  platforms  = ["linux/amd64", "linux/arm64"]
  cache-from = ["type=gha"]
  cache-to   = ["type=gha,mode=max"]
}

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
