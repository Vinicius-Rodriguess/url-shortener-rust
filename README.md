# 🔗 URL Shortener (Rust + Axum + Redis + Cassandra)

Este projeto é um **encurtador de URLs** altamente escalável, desenvolvido em **Rust** 🦀, utilizando o framework **Axum** no back-end. A aplicação implementa um algoritmo de **Base62 com ofuscação determinística via HashID**, gerando códigos curtos únicos e imprevisíveis a partir de um contador global distribuído gerenciado no **Redis**, com armazenamento persistente no **Cassandra (ScyllaDB)**.

-----

## 🚀 Funcionalidades

### Back-End

  * Geração de URLs curtas via **Base62 ofuscado** com chave secreta (`SECRET_KEY`).
  * Armazenamento persistente dos links no **Cassandra/ScyllaDB**.
  * Gerenciamento distribuído de IDs sequenciais com **Redis INCR**.
  * Redirecionamento automático (`302 Found`) ao acessar uma URL encurtada.
  * Projeto **escalável e resiliente**, ideal para múltiplas instâncias.

-----

## 🧠 Como o Sistema Funciona

O encurtamento segue um fluxo matemático e criptográfico simples e eficiente. A geração do código curto ocorre em **cinco etapas principais**:

### 1️⃣ Base62 com Ofuscação (HashID-like)

O alfabeto padrão Base62 (`a-zA-Z0-9`) é **embaralhado de forma determinística** com base em uma `SECRET_KEY`. O embaralhamento usa `blake3` como hash e `ChaCha8Rng` como semente. Assim, cada instância com a mesma chave gera o mesmo padrão de embaralhamento — garantindo unicidade e previsibilidade controlada.

### 2️⃣ Número incremental com Redis

O **Redis** atua como **contador global** via `INCR url_id`. Esse contador é compartilhado entre todas as instâncias da aplicação, garantindo **IDs únicos**. O ID inicial é deslocado em `+14.000.000` para garantir que todas as URLs tenham pelo menos **4 caracteres** (medida de segurança e estética).

### 3️⃣ Conversão de ID → Base62

O número gerado pelo Redis é dividido por 62 repetidamente até não ser mais possível. Cada resto da divisão é convertido para um caractere do alfabeto Base62 ofuscado.

**Exemplo:**

```
11157 / 62 = 179 (resto 59)
179 / 62 = 2 (resto 55)
2 / 62 = 0 (resto 2)
→ restos: [2, 55, 59]
→ caracteres: 2, t, x
→ short_url: 2tx
```

### 4️⃣ Dicionário Base62 embaralhado

O mapeamento dos restos para caracteres é feito com base no **alfabeto ofuscado** gerado a partir da `SECRET_KEY`, garantindo que o mesmo número produza sempre o mesmo código curto — mas não sequencial.

### 5️⃣ Armazenamento no Cassandra

Após gerar o `short_url`, o sistema executa a *query*:

```yaml
INSERT INTO shortener.urls (short_url, long_url) VALUES (?, ?);
```

Os dados são gravados de forma distribuída, garantindo alta disponibilidade e consistência eventual.

-----

## 💾 Arquitetura de Persistência

  * **Redis** → gera IDs únicos e globais (`INCR`).
  * **Cassandra/Scylla** → armazena pares `{ short_url, long_url }`.
  * Cada novo link recebe um **ID inteiro exclusivo** que é convertido em Base62 e ofuscado.

-----

## 🧩 Tecnologias Utilizadas

| Categoria | Tecnologia |
| :---------- | :---------- |
| Linguagem | **Rust** 🦀 |
| Framework Web | **Axum** |
| Banco de Dados | **Cassandra / ScyllaDB** |
| Cache / Contador | **Redis** |
| Hash e RNG | **blake3**, **rand\_chacha** |
| ORM/Driver | **scylla-rs** |
| Execução assíncrona | **Tokio** |
| Containerização | **Docker & Docker Compose** |

-----

## 🧱 Estrutura de Diretórios

```yaml
url-shortener-rust/
├── src/
│ ├── main.rs # Código principal (Axum, Redis, Cassandra)
│ └── ...
├── Dockerfile # Build multi-stage para backend Rust
├── docker-compose.yml # Orquestração: backend + Redis + Scylla
└── README.md # Documentação completa
```

-----

## 🐳 Execução com Docker

1.  Suba os serviços:

    ```markdown
    docker compose up --build
    ```

2.  Aguarde o Cassandra inicializar e criar automaticamente o keyspace e tabela.

3.  Acesse a API:

    ```yaml
    http://localhost:3000
    ```

-----

## 🔗 Endpoints

### `POST /shorten`

**Cria uma nova URL encurtada**

📤 **Request:**

```markdown
{
"long_url": "https://rust-lang.org"
}
```

📥 **Response:**

```yaml
{
"short_url": "2tx",
"long_url": "https://rust-lang.org"
}
```

-----

### `GET /:short_url`

**Redireciona para a URL original**

📥 **Exemplo:**

```yaml
GET /2tx
→ 302 Found
Location: https://rust-lang.org
```

-----

## ⚙️ Configuração via Variáveis de Ambiente

| Variável | Descrição | Exemplo |
| :---------- | :---------- | :---------- |
| `SECRET_KEY` | Chave para embaralhar o alfabeto Base62 | `"minha_chave_segura"` |
| `REDIS_URL` | URL de conexão do Redis | `"redis://redis:6379/"` |
| `CASSANDRA_HOST` | Host Cassandra (ou Scylla) | `"cassandra"` |

-----

## 📈 Escalabilidade

  * O uso do **Redis** como contador global permite **múltiplas instâncias simultâneas** sem colisões.
  * O **Cassandra** garante **replicação, tolerância a falhas e escrita distribuída**.
  * Arquitetura **stateless**: o backend não guarda estado local — ideal para **deploy em clusters** (Kubernetes, Swarm, etc).

-----

## ⚡ Pontos de Segurança e Boas Práticas

  * A ofuscação via `SECRET_KEY` impede predição direta das URLs.
  * IDs sempre exclusivos, sem colisão, gerados pelo Redis.
  * A aplicação **não aceita duplicidade** de `short_url`.
  * Utilize `SECRET_KEY` única por ambiente.
  * TLS recomendado para comunicação entre serviços.

-----

## 🧩 Diagramas e Referências

  * 🎥 **Vídeo base do estudo (Renato Augusto):**
    [https://youtu.be/m\_anIoKW7Jg?si=2EuiZwdMeRo1-gej](https://youtu.be/m_anIoKW7Jg?si=2EuiZwdMeRo1-gej)

  * 🧭 **Diagramas e arquitetura (Miro):**
    [https://miro.com/app/board/uXjVJ0kAdLs=/](https://miro.com/app/board/uXjVJ0kAdLs=/)

-----

## 🔮 Melhorias Futuras

  * Implementar endpoint de estatísticas (número de acessos por short).
  * Adicionar cache de redirecionamento com TTL em Redis.
  * Autenticação com API Key para criação de URLs.
  * Testes automatizados (unit e integração).
  * Rate limiting e logs estruturados.