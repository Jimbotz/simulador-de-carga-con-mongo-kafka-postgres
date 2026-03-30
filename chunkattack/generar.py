import csv
import os
import time
import random
import logging
from faker_persona_mx import PersonaGenerator

logging.getLogger("faker_persona_mx").setLevel(logging.CRITICAL)

ARCHIVO_SALIDA = "/data/usuarios_sinteticos.csv"
TOTAL_REGISTROS = 1_000_000 # dos millones de registros, eso se puede cambiar aca (1000000)
TAMAÑO_LOTE    = 10_000


def ya_existe():
    """Devuelve True si el CSV ya fue generado en una ejecución anterior."""
    if not os.path.isfile(ARCHIVO_SALIDA):
        return False
    tamaño = os.path.getsize(ARCHIVO_SALIDA)
    # Si pesa menos de 1 MB asumimos que está incompleto o vacío
    return tamaño > 1_000_000


def crear_dataset_masivo():
    if ya_existe():
        print(f" CSV ya existe en {ARCHIVO_SALIDA} — saltando generación.")
        return

    os.makedirs(os.path.dirname(ARCHIVO_SALIDA), exist_ok=True)

    generator = PersonaGenerator()
    print(f"Iniciando generación de {TOTAL_REGISTROS:,} registros en {ARCHIVO_SALIDA}...")
    tiempo_inicio = time.time()

    with open(ARCHIVO_SALIDA, mode="w", newline="", encoding="utf-8") as archivo:
        writer = csv.writer(archivo)
        writer.writerow([
            "nombre", "apellido_paterno", "apellido_materno",
            "curp", "rfc", "email", "telefono", "edad"
        ])

        lotes_necesarios = TOTAL_REGISTROS // TAMAÑO_LOTE

        for i in range(lotes_necesarios):
            lote_personas = generator.generate_batch(TAMAÑO_LOTE)
            filas = [
                [
                    p.nombre, p.apellido_paterno, p.apellido_materno,
                    p.curp, p.rfc, p.email, p.telefono,
                    random.randint(15, 90)
                ]
                for p in lote_personas
            ]
            writer.writerows(filas)

            if (i + 1) % 10 == 0:
                progreso = (i + 1) * TAMAÑO_LOTE
                print(f"Progreso: {progreso:,} / {TOTAL_REGISTROS:,} registros guardados...")

    tiempo_total = time.time() - tiempo_inicio
    print(f" ¡Completado! {ARCHIVO_SALIDA} generado en {tiempo_total:.2f}s")


if __name__ == "__main__":
    crear_dataset_masivo()
