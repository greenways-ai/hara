package hara.kernel.maven;

import org.apache.maven.repository.internal.MavenRepositorySystemUtils;
import org.eclipse.aether.RepositorySystem;
import org.eclipse.aether.connector.basic.BasicRepositoryConnectorFactory;
import org.eclipse.aether.impl.DefaultServiceLocator;
import org.eclipse.aether.spi.connector.RepositoryConnectorFactory;
import org.eclipse.aether.spi.connector.transport.TransporterFactory;
import org.eclipse.aether.transport.http.HttpTransporterFactory;

public class RepositorySystemFactory {

  public static RepositorySystem newRepositorySystem() {
    DefaultServiceLocator locator = MavenRepositorySystemUtils.newServiceLocator();
    locator.addService(RepositoryConnectorFactory.class, BasicRepositoryConnectorFactory.class);
    locator.addService(TransporterFactory.class, HttpTransporterFactory.class);

    locator.setErrorHandler(
        new DefaultServiceLocator.ErrorHandler() {
          @Override
          public void serviceCreationFailed(Class<?> type, Class<?> impl, Throwable exception) {
            exception.printStackTrace();
          }
        });

    return locator.getService(RepositorySystem.class);
  }
}
