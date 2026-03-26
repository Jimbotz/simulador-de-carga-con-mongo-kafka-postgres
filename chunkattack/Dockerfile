FROM python:3.12-slim

# ─── Configurar UTF-8 por defecto para evitar caracteres corruptos ──────
ENV LANG=C.UTF-8
ENV LC_ALL=C.UTF-8
ENV PYTHONIOENCODING=utf-8

WORKDIR /app

# Dependencias del sistema: git para clonar el repo, gcc para compilar extensiones C
RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    gcc \
    && rm -rf /var/lib/apt/lists/*

# ── Instalar faker-persona-mx desde el código fuente ─────────────────────────
# Se clona el repo y se instala en modo editable (pip install -e .)
# Esto garantiza usar el código actual del repositorio, incluyendo el fix
# de rutas de la versión 2.0.3+ que corrige FileNotFoundError en paquetes instalados.
# No se necesita venv: el contenedor ya es un ambiente aislado.
RUN git clone https://github.com/pedro-aaron/faker-persona-mx.git /tmp/faker-persona-mx \
    && pip install --no-cache-dir -e /tmp/faker-persona-mx

# ── Script generador ──────────────────────────────────────────────────────────
COPY generar.py .

# El CSV se escribirá en /data — volumen compartido con el contenedor simulador
VOLUME ["/data"]

CMD ["python", "generar.py"]
