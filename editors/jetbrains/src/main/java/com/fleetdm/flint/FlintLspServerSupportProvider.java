package com.fleetdm.flint;

import com.intellij.openapi.project.Project;
import com.intellij.openapi.vfs.VirtualFile;
import com.intellij.platform.lsp.api.LspServerSupportProvider;
import com.intellij.platform.lsp.api.LspServer;
import org.jetbrains.annotations.NotNull;

/**
 * Registers the Flint LSP server for Fleet GitOps YAML files.
 *
 * Activates when editing YAML files in projects that contain Fleet GitOps
 * indicators (default.yml, fleets/, .fleetlint.toml).
 */
public class FlintLspServerSupportProvider implements LspServerSupportProvider {

    @Override
    public void fileOpened(@NotNull Project project, @NotNull VirtualFile file, @NotNull LspServerStarter serverStarter) {
        if (!isFleetYaml(project, file)) {
            return;
        }

        serverStarter.ensureServerStarted(new FlintLspServerDescriptor(project));
    }

    private boolean isFleetYaml(Project project, VirtualFile file) {
        String name = file.getName();
        if (!name.endsWith(".yml") && !name.endsWith(".yaml")) {
            return false;
        }

        // Check for Fleet GitOps indicators in the project
        VirtualFile projectDir = project.getBaseDir();
        if (projectDir == null) return false;

        return projectDir.findChild("default.yml") != null
            || projectDir.findChild("default.yaml") != null
            || projectDir.findChild("fleets") != null
            || projectDir.findChild(".fleetlint.toml") != null;
    }
}
