package hara.kernel.maven;

import org.apache.maven.repository.internal.MavenRepositorySystemUtils;
import org.eclipse.aether.DefaultRepositorySystemSession;
import org.eclipse.aether.RepositorySystem;
import org.eclipse.aether.artifact.Artifact;
import org.eclipse.aether.artifact.DefaultArtifact;
import org.eclipse.aether.collection.CollectRequest;
import org.eclipse.aether.graph.Dependency;
import org.eclipse.aether.repository.LocalRepository;
import org.eclipse.aether.repository.RemoteRepository;
import org.eclipse.aether.resolution.ArtifactResult;
import org.eclipse.aether.resolution.DependencyRequest;
import org.eclipse.aether.resolution.DependencyResolutionException;

import java.io.File;
import java.util.ArrayList;
import java.util.Collection;
import java.util.Comparator;
import java.util.LinkedHashMap;
import java.util.List;

public class MavenResolver {

  private final RepositorySystem system;
  private final DefaultRepositorySystemSession session;
  private final List<RemoteRepository> repositories;

  public MavenResolver() {
    this(false);
  }

  public MavenResolver(boolean offline) {
    this.system = RepositorySystemFactory.newRepositorySystem();
    this.session = MavenRepositorySystemUtils.newSession();
    LocalRepository localRepo =
        new LocalRepository(System.getProperty("user.home") + "/.m2/repository");
    this.session.setLocalRepositoryManager(
        this.system.newLocalRepositoryManager(this.session, localRepo));
    this.session.setOffline(offline);
    this.repositories =
        List.of(
            new RemoteRepository.Builder(
                    "central", "default", "https://repo.maven.apache.org/maven2/")
                .build());
  }

  public List<File> resolve(String coordinate) throws DependencyResolutionException {
    return resolve(List.of(coordinate));
  }

  /** Resolves all roots as one graph so Maven can mediate shared transitive dependencies. */
  public List<File> resolve(Collection<String> coordinates)
      throws DependencyResolutionException {
    CollectRequest collectRequest = new CollectRequest();
    for (String coordinate : coordinates) {
      Artifact artifact = new DefaultArtifact(coordinate);
      collectRequest.addDependency(new Dependency(artifact, "compile"));
    }
    collectRequest.setRepositories(this.repositories);

    DependencyRequest dependencyRequest = new DependencyRequest();
    dependencyRequest.setCollectRequest(collectRequest);

    List<ArtifactResult> artifactResults =
        this.system.resolveDependencies(this.session, dependencyRequest).getArtifactResults();

    LinkedHashMap<String, File> files = new LinkedHashMap<>();
    artifactResults.stream()
        .map(ArtifactResult::getArtifact)
        .map(Artifact::getFile)
        .filter(java.util.Objects::nonNull)
        .sorted(Comparator.comparing(File::getAbsolutePath))
        .forEach(file -> files.put(file.getAbsolutePath(), file));
    return List.copyOf(new ArrayList<>(files.values()));
  }
}
