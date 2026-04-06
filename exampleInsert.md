**Ejemplo de nuevo registro**

Debe de segir este formato:

``` bash
db.getSiblingDB("SyntheticDB").Users.insertOne({
  _id: "usuario-caos-001",
  nombre: "Jayme",
  estatus: "activo",
  
  // TRAMPA 1: Arreglo de Tipos Mixtos (Entero, texto, booleano, decimal)
  datos_mixtos: [1, "dos", true, 3.14],
  
  // TRAMPA 2: Arreglo de Objetos (El destructor de conectores)
  historial_compras: [
    { "item": "Monitor", "precio": 4000 },
    { "item": "Teclado", "precio": 800 }
  ],
  
  // TRAMPA 3: Explosión de Columnas (Diccionario con llaves dinámicas UUID)
  sesiones_activas: {
    "uuid-1234-abcd": { "duracion_min": 15 },
    "uuid-9876-zyxw": { "duracion_min": 42 }
  },
  
  // TRAMPA 4, 5 y 6: Anidación profunda (> 63 caracteres), Objeto Vacío y Llave con Punto
  // Todo agrupado en un solo objeto padre para neutralizarlo.
  metadata_compleja: {
    configuracion_avanzada_del_sistema_notificaciones_email_frecuencia_lunes: true,
    preferencias_vacias: {},
    "servicios.premium": "activado"
  },
  
  // CASO NORMAL: Objetos que SÍ queremos que se aplanen en columnas separadas
  direccion: {
    calle: "Av. Independencia",
    municipio: "Ocotlán",
    estado: "Tlaxcala",
    cp: "90000"
  },
  
  // NUEVA COLUMNA: caso_especial con estructura compleja
  caso_especial: {
    preferencias: ["a", "b", "d"],  // Nota: corregido a arreglo, no múltiples valores
    nivel_1: "nvel 1",
    subniveles: {
      nivel_1: {
        subnivel_1: {}
      }
    }
  }
});
```


# Campos ignorados
`apellido_materno`, `apellido_paterno`, `curp`, `edad`, `email`, `rfc`, `telefono`, `deleted_at`