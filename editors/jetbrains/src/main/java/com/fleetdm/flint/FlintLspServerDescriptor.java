package com.fleetdm.flint;

import com.intellij.execution.configurations.GeneralCommandLine;
import com.intellij.openapi.project.Project;
import com.intellij.openapi.vfs.VirtualFile;
import com.intellij.platform.lsp.api.ProjectWideLspServerDescriptor;
import org.jetbrains.annotations.NotNull;

/**
 * Describes how to launch the Flint LSP server.
 */
public class FlintLspServerDescriptor extends ProjectWideLspServerDescriptor {

    public FlintLspServerDescriptor(@NotNull Project project) {
        super(project, "Flint");
    }

    @NotNull
    @Override
    public GeneralCommandLine createCommandLine() {
        GeneralCommandLine cmd = new GeneralCommandLine("flint", "lsp");
        cmd.withWorkDirectory(getProject().getBasePath());
        return cmd;
    }

    @Override
    public boolean isSupportedFile(@NotNull VirtualFile file) {
        String name = file.getName();
        if (!name.endsWith(".yml") && !name.endsWith(".yaml")) {
            return false;
        }

        String path = file.getPath();
        return path.contains("/fleets/")
            || path.contains("/teams/")
            || path.contains("/lib/")
            || path.contains("/platforms/")
            || path.contains("/labels/")
            || name.equals("default.yml")
            || name.equals("default.yaml");
    }
}
