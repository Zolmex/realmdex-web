FROM php:8.2-apache-bookworm

# Install SQLite and necessary extensions
RUN apt-get update && apt-get install -y \
    sqlite3 \
    libsqlite3-dev \
    && docker-php-ext-install pdo pdo_sqlite \
    && rm -rf /var/lib/apt/lists/*

# Enable Apache rewrite module
RUN a2enmod rewrite
RUN echo "ServerName localhost" >> /etc/apache2/apache2.conf

# Secure PHP settings
RUN mv "$PHP_INI_DIR/php.ini-production" "$PHP_INI_DIR/php.ini" && \
    sed -i 's/expose_php = On/expose_php = Off/' "$PHP_INI_DIR/php.ini" && \
    sed -i 's/ServerTokens OS/ServerTokens Prod/' /etc/apache2/conf-available/security.conf && \
    sed -i 's/ServerSignature On/ServerSignature Off/' /etc/apache2/conf-available/security.conf

# Set working directory
WORKDIR /var/www/html

# Copy project files
COPY . .

# Create directory for SQLite database and set permissions
RUN mkdir -p /var/www/html/data && \
    chown -R www-data:www-data /var/www/html && \
    chmod +x /var/www/html/entrypoint.sh

# Environment variable for DB path (matches the code audit suggestion)
ENV DB_PATH=/var/www/html/data/uptime.db

# Expose port 80
EXPOSE 80

# Use the entrypoint script to start polling and Apache
ENTRYPOINT ["sh", "/var/www/html/entrypoint.sh"]
