FROM openjdk:21-jdk-slim

RUN mkdir /opt/app
COPY target/hara.lang-0.0.1-SNAPSHOT.jar /opt/app/kernel.jar
CMD ["java", "-jar", "/opt/app/kernel.jar"]

# Ports used by the app
EXPOSE 4164