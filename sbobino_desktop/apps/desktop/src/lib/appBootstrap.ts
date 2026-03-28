import type {
  AppSettings,
  ProvisioningModelCatalogEntry,
  ProvisioningStatus,
  TranscriptArtifact,
} from "../types";

export type AppBootstrapLoaders = {
  fetchSettingsSnapshot: () => Promise<AppSettings>;
  listRecentArtifacts: () => Promise<TranscriptArtifact[]>;
  listDeletedArtifacts: (payload: { limit: number }) => Promise<TranscriptArtifact[]>;
  provisioningStatus: () => Promise<ProvisioningStatus>;
  provisioningModels: () => Promise<ProvisioningModelCatalogEntry[]>;
};

export type InitialAppBootstrapOptions = {
  standaloneSettingsWindow: boolean;
  includeActiveArtifacts?: boolean;
  includeDeletedArtifacts?: boolean;
  includeProvisioning?: boolean;
  includeModelCatalog?: boolean;
};

export type InitialAppBootstrapData = {
  settings: AppSettings;
  activeArtifacts: TranscriptArtifact[] | null;
  deletedArtifacts: TranscriptArtifact[] | null;
  provisioning: ProvisioningStatus | null;
  modelCatalog: ProvisioningModelCatalogEntry[] | null;
};

export async function loadInitialAppBootstrapData(
  loaders: AppBootstrapLoaders,
  options: InitialAppBootstrapOptions,
): Promise<InitialAppBootstrapData> {
  const {
    standaloneSettingsWindow,
    includeActiveArtifacts = !options.standaloneSettingsWindow,
    includeDeletedArtifacts = false,
    includeProvisioning = false,
    includeModelCatalog = false,
  } = options;
  const settings = await loaders.fetchSettingsSnapshot();

  const activeArtifactsPromise: Promise<TranscriptArtifact[] | null> = !includeActiveArtifacts
    ? Promise.resolve(null)
    : standaloneSettingsWindow
    ? Promise.resolve(null)
    : loaders.listRecentArtifacts();
  const deletedArtifactsPromise: Promise<TranscriptArtifact[] | null> = includeDeletedArtifacts
    ? loaders.listDeletedArtifacts({ limit: 200 })
    : Promise.resolve(null);
  const provisioningPromise: Promise<ProvisioningStatus | null> = includeProvisioning
    ? loaders.provisioningStatus()
    : Promise.resolve(null);
  const modelCatalogPromise: Promise<ProvisioningModelCatalogEntry[] | null> = includeModelCatalog
    ? loaders.provisioningModels()
    : Promise.resolve(null);

  const [
    activeArtifactsResult,
    deletedArtifactsResult,
    provisioningResult,
    modelCatalogResult,
  ] = await Promise.allSettled([
    activeArtifactsPromise,
    deletedArtifactsPromise,
    provisioningPromise,
    modelCatalogPromise,
  ]);

  return {
    settings,
    activeArtifacts: activeArtifactsResult.status === "fulfilled"
      ? activeArtifactsResult.value
      : null,
    deletedArtifacts: deletedArtifactsResult.status === "fulfilled"
      ? deletedArtifactsResult.value
      : null,
    provisioning: provisioningResult.status === "fulfilled"
      ? provisioningResult.value
      : null,
    modelCatalog: modelCatalogResult.status === "fulfilled"
      ? modelCatalogResult.value
      : null,
  };
}
