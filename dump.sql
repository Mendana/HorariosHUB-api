--
-- PostgreSQL database dump
--

\restrict nAIoWOCwuApuxoCClTlwoOOtc0EDzj7aYByJDPwEohq0lXOWu1KuPub9qo7twfj

-- Dumped from database version 16.14
-- Dumped by pg_dump version 16.14

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: pgcrypto; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public;


--
-- Name: EXTENSION pgcrypto; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION pgcrypto IS 'cryptographic functions';


--
-- Name: change_status; Type: TYPE; Schema: public; Owner: horariosUser
--

CREATE TYPE public.change_status AS ENUM (
    'pending',
    'approved',
    'rejected'
);


ALTER TYPE public.change_status OWNER TO "horariosUser";

--
-- Name: change_type; Type: TYPE; Schema: public; Owner: horariosUser
--

CREATE TYPE public.change_type AS ENUM (
    'create',
    'modify',
    'delete'
);


ALTER TYPE public.change_type OWNER TO "horariosUser";

--
-- Name: user_role; Type: TYPE; Schema: public; Owner: horariosUser
--

CREATE TYPE public.user_role AS ENUM (
    'admin',
    'professor',
    'student'
);


ALTER TYPE public.user_role OWNER TO "horariosUser";

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: _sqlx_migrations; Type: TABLE; Schema: public; Owner: horariosUser
--

CREATE TABLE public._sqlx_migrations (
    version bigint NOT NULL,
    description text NOT NULL,
    installed_on timestamp with time zone DEFAULT now() NOT NULL,
    success boolean NOT NULL,
    checksum bytea NOT NULL,
    execution_time bigint NOT NULL
);


ALTER TABLE public._sqlx_migrations OWNER TO "horariosUser";

--
-- Name: changes; Type: TABLE; Schema: public; Owner: horariosUser
--

CREATE TABLE public.changes (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    proposed_by uuid NOT NULL,
    session_id uuid,
    subject text,
    grp text,
    change_type public.change_type NOT NULL,
    change_status public.change_status DEFAULT 'pending'::public.change_status NOT NULL,
    prev_starts_at timestamp with time zone,
    prev_duration integer,
    new_starts_at timestamp with time zone,
    new_duration integer,
    proposed_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT chk_create CHECK (((change_type <> 'create'::public.change_type) OR ((session_id IS NULL) AND (subject IS NOT NULL) AND (grp IS NOT NULL)))),
    CONSTRAINT chk_delete CHECK (((change_type <> 'delete'::public.change_type) OR ((session_id IS NOT NULL) AND (subject IS NULL) AND (grp IS NULL)))),
    CONSTRAINT chk_modify CHECK (((change_type <> 'modify'::public.change_type) OR ((session_id IS NOT NULL) AND (subject IS NULL) AND (grp IS NULL))))
);


ALTER TABLE public.changes OWNER TO "horariosUser";

--
-- Name: password_reset_tokens; Type: TABLE; Schema: public; Owner: horariosUser
--

CREATE TABLE public.password_reset_tokens (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    token text NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.password_reset_tokens OWNER TO "horariosUser";

--
-- Name: schedule; Type: TABLE; Schema: public; Owner: horariosUser
--

CREATE TABLE public.schedule (
    user_id uuid NOT NULL,
    subject text NOT NULL,
    grp text NOT NULL
);


ALTER TABLE public.schedule OWNER TO "horariosUser";

--
-- Name: sessions; Type: TABLE; Schema: public; Owner: horariosUser
--

CREATE TABLE public.sessions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    subject text NOT NULL,
    grp text NOT NULL,
    starts_at timestamp with time zone NOT NULL,
    duration_min integer NOT NULL,
    is_overridden boolean DEFAULT false NOT NULL,
    scraped_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.sessions OWNER TO "horariosUser";

--
-- Name: subject_groups; Type: TABLE; Schema: public; Owner: horariosUser
--

CREATE TABLE public.subject_groups (
    subject text NOT NULL,
    grp text NOT NULL
);


ALTER TABLE public.subject_groups OWNER TO "horariosUser";

--
-- Name: users; Type: TABLE; Schema: public; Owner: horariosUser
--

CREATE TABLE public.users (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    email text NOT NULL,
    role public.user_role NOT NULL,
    verified boolean DEFAULT false NOT NULL,
    password_hash text NOT NULL
);


ALTER TABLE public.users OWNER TO "horariosUser";

--
-- Name: verification_tokens; Type: TABLE; Schema: public; Owner: horariosUser
--

CREATE TABLE public.verification_tokens (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    token text NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.verification_tokens OWNER TO "horariosUser";

--
-- Data for Name: _sqlx_migrations; Type: TABLE DATA; Schema: public; Owner: horariosUser
--

COPY public._sqlx_migrations (version, description, installed_on, success, checksum, execution_time) FROM stdin;
20260516114810	initial schema	2026-05-17 16:37:21.350834+00	t	\\x38451db30f6eed79d911c16ca5b6df32c01242f74656afa9f7b43be8b569126b7e50c1eb792075c258a6eea4c2d10b04	136162400
20260516183710	add password to users	2026-05-17 16:37:21.490165+00	t	\\xdcee9c3364023dc8e99cccba3dcb1a6b7b0cebe16a534b85dfb6bb07b60313f5ff25b2f5b15ff76d89e28bb084cf8486	3391800
20260517073150	add verification tokens	2026-05-17 16:37:21.495392+00	t	\\xb14f6f23790d3d6c5d2a7b245ecffd3f71066702643b6bc9456a5eded0b17b93a7e076f775a9f6b8b6a691e978273921	44936700
20260517095458	add password reset tokens	2026-05-17 16:37:21.542101+00	t	\\xeba2162178346c731c86bed58740f8e071b005b00c560f94cd5b2bf95d2437527d7f57c530408cae97c12ed3c34f6e9b	15671200
\.


--
-- Data for Name: changes; Type: TABLE DATA; Schema: public; Owner: horariosUser
--

COPY public.changes (id, proposed_by, session_id, subject, grp, change_type, change_status, prev_starts_at, prev_duration, new_starts_at, new_duration, proposed_at) FROM stdin;
\.


--
-- Data for Name: password_reset_tokens; Type: TABLE DATA; Schema: public; Owner: horariosUser
--

COPY public.password_reset_tokens (id, user_id, token, expires_at, created_at) FROM stdin;
\.


--
-- Data for Name: schedule; Type: TABLE DATA; Schema: public; Owner: horariosUser
--

COPY public.schedule (user_id, subject, grp) FROM stdin;
\.


--
-- Data for Name: sessions; Type: TABLE DATA; Schema: public; Owner: horariosUser
--

COPY public.sessions (id, subject, grp, starts_at, duration_min, is_overridden, scraped_at) FROM stdin;
\.


--
-- Data for Name: subject_groups; Type: TABLE DATA; Schema: public; Owner: horariosUser
--

COPY public.subject_groups (subject, grp) FROM stdin;
\.


--
-- Data for Name: users; Type: TABLE DATA; Schema: public; Owner: horariosUser
--

COPY public.users (id, email, role, verified, password_hash) FROM stdin;
23fa8ced-1fd2-4c06-bb3a-3a0338f32b4d	alumno@uniovi.es	student	t	$2b$12$okiHTXQU7IDPdpES5y9rIe1DwGVDm4n6H3hU13ORFRvvrjUbYOoVO
2423d570-5b88-4919-a3b6-036e93db57a6	admin@uniovi.es	admin	t	$2b$12$ZJ52vYCBj3YFjaaGBZ6g6uO0eFc2s4mVyrxrIjkz3lvdXJ3xmfPwO
76f543ee-1191-4f4a-91eb-f571a3654de1	profesor@uniovi.es	professor	t	$2b$12$79DnqZDHeMAkquhjZE.v0uNDs3G9YlTxy1obw2vWJsscmSPlO..QW
\.


--
-- Data for Name: verification_tokens; Type: TABLE DATA; Schema: public; Owner: horariosUser
--

COPY public.verification_tokens (id, user_id, token, expires_at, created_at) FROM stdin;
8f810608-da04-4599-a1c8-21b1075b66af	23fa8ced-1fd2-4c06-bb3a-3a0338f32b4d	71a01a50-46d9-4a2e-a841-fb0dfb5008ca	2026-05-18 16:38:22.118143+00	2026-05-17 16:38:22.118143+00
ae5e771a-e91a-4608-8da7-b73ffed7bf84	2423d570-5b88-4919-a3b6-036e93db57a6	f8cede2c-249d-46d9-af71-120aaec86c3a	2026-05-18 16:38:28.261986+00	2026-05-17 16:38:28.261986+00
6276e02c-9cab-4fcd-b58f-8e840c4c8541	76f543ee-1191-4f4a-91eb-f571a3654de1	e4ac5c4e-abed-4550-9b0c-52553bc5ab97	2026-05-18 16:38:55.609315+00	2026-05-17 16:38:55.609315+00
\.


--
-- Name: _sqlx_migrations _sqlx_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: horariosUser
--

ALTER TABLE ONLY public._sqlx_migrations
    ADD CONSTRAINT _sqlx_migrations_pkey PRIMARY KEY (version);


--
-- Name: changes changes_pkey; Type: CONSTRAINT; Schema: public; Owner: horariosUser
--

ALTER TABLE ONLY public.changes
    ADD CONSTRAINT changes_pkey PRIMARY KEY (id);


--
-- Name: password_reset_tokens password_reset_tokens_pkey; Type: CONSTRAINT; Schema: public; Owner: horariosUser
--

ALTER TABLE ONLY public.password_reset_tokens
    ADD CONSTRAINT password_reset_tokens_pkey PRIMARY KEY (id);


--
-- Name: password_reset_tokens password_reset_tokens_token_key; Type: CONSTRAINT; Schema: public; Owner: horariosUser
--

ALTER TABLE ONLY public.password_reset_tokens
    ADD CONSTRAINT password_reset_tokens_token_key UNIQUE (token);


--
-- Name: schedule schedule_pkey; Type: CONSTRAINT; Schema: public; Owner: horariosUser
--

ALTER TABLE ONLY public.schedule
    ADD CONSTRAINT schedule_pkey PRIMARY KEY (user_id, subject, grp);


--
-- Name: sessions sessions_pkey; Type: CONSTRAINT; Schema: public; Owner: horariosUser
--

ALTER TABLE ONLY public.sessions
    ADD CONSTRAINT sessions_pkey PRIMARY KEY (id);


--
-- Name: sessions sessions_subject_grp_starts_at_key; Type: CONSTRAINT; Schema: public; Owner: horariosUser
--

ALTER TABLE ONLY public.sessions
    ADD CONSTRAINT sessions_subject_grp_starts_at_key UNIQUE (subject, grp, starts_at);


--
-- Name: subject_groups subject_groups_pkey; Type: CONSTRAINT; Schema: public; Owner: horariosUser
--

ALTER TABLE ONLY public.subject_groups
    ADD CONSTRAINT subject_groups_pkey PRIMARY KEY (subject, grp);


--
-- Name: users users_email_key; Type: CONSTRAINT; Schema: public; Owner: horariosUser
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_email_key UNIQUE (email);


--
-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: horariosUser
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


--
-- Name: verification_tokens verification_tokens_pkey; Type: CONSTRAINT; Schema: public; Owner: horariosUser
--

ALTER TABLE ONLY public.verification_tokens
    ADD CONSTRAINT verification_tokens_pkey PRIMARY KEY (id);


--
-- Name: verification_tokens verification_tokens_token_key; Type: CONSTRAINT; Schema: public; Owner: horariosUser
--

ALTER TABLE ONLY public.verification_tokens
    ADD CONSTRAINT verification_tokens_token_key UNIQUE (token);


--
-- Name: idx_changes_session_id; Type: INDEX; Schema: public; Owner: horariosUser
--

CREATE INDEX idx_changes_session_id ON public.changes USING btree (session_id);


--
-- Name: idx_changes_status_proposed_at; Type: INDEX; Schema: public; Owner: horariosUser
--

CREATE INDEX idx_changes_status_proposed_at ON public.changes USING btree (change_status, proposed_at DESC);


--
-- Name: idx_password_reset_tokens_token; Type: INDEX; Schema: public; Owner: horariosUser
--

CREATE INDEX idx_password_reset_tokens_token ON public.password_reset_tokens USING btree (token);


--
-- Name: idx_password_reset_tokens_user_id; Type: INDEX; Schema: public; Owner: horariosUser
--

CREATE INDEX idx_password_reset_tokens_user_id ON public.password_reset_tokens USING btree (user_id);


--
-- Name: idx_sessions_overridden; Type: INDEX; Schema: public; Owner: horariosUser
--

CREATE INDEX idx_sessions_overridden ON public.sessions USING btree (id) WHERE (is_overridden = true);


--
-- Name: idx_sessions_starts_at; Type: INDEX; Schema: public; Owner: horariosUser
--

CREATE INDEX idx_sessions_starts_at ON public.sessions USING btree (starts_at);


--
-- Name: idx_verification_tokens_token; Type: INDEX; Schema: public; Owner: horariosUser
--

CREATE INDEX idx_verification_tokens_token ON public.verification_tokens USING btree (token);


--
-- Name: idx_verification_tokens_user; Type: INDEX; Schema: public; Owner: horariosUser
--

CREATE INDEX idx_verification_tokens_user ON public.verification_tokens USING btree (user_id);


--
-- Name: changes changes_proposed_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: horariosUser
--

ALTER TABLE ONLY public.changes
    ADD CONSTRAINT changes_proposed_by_fkey FOREIGN KEY (proposed_by) REFERENCES public.users(id);


--
-- Name: changes changes_session_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: horariosUser
--

ALTER TABLE ONLY public.changes
    ADD CONSTRAINT changes_session_id_fkey FOREIGN KEY (session_id) REFERENCES public.sessions(id);


--
-- Name: changes changes_subject_grp_fkey; Type: FK CONSTRAINT; Schema: public; Owner: horariosUser
--

ALTER TABLE ONLY public.changes
    ADD CONSTRAINT changes_subject_grp_fkey FOREIGN KEY (subject, grp) REFERENCES public.subject_groups(subject, grp);


--
-- Name: password_reset_tokens password_reset_tokens_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: horariosUser
--

ALTER TABLE ONLY public.password_reset_tokens
    ADD CONSTRAINT password_reset_tokens_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: schedule schedule_subject_grp_fkey; Type: FK CONSTRAINT; Schema: public; Owner: horariosUser
--

ALTER TABLE ONLY public.schedule
    ADD CONSTRAINT schedule_subject_grp_fkey FOREIGN KEY (subject, grp) REFERENCES public.subject_groups(subject, grp);


--
-- Name: schedule schedule_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: horariosUser
--

ALTER TABLE ONLY public.schedule
    ADD CONSTRAINT schedule_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: sessions sessions_subject_grp_fkey; Type: FK CONSTRAINT; Schema: public; Owner: horariosUser
--

ALTER TABLE ONLY public.sessions
    ADD CONSTRAINT sessions_subject_grp_fkey FOREIGN KEY (subject, grp) REFERENCES public.subject_groups(subject, grp);


--
-- Name: verification_tokens verification_tokens_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: horariosUser
--

ALTER TABLE ONLY public.verification_tokens
    ADD CONSTRAINT verification_tokens_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--

\unrestrict nAIoWOCwuApuxoCClTlwoOOtc0EDzj7aYByJDPwEohq0lXOWu1KuPub9qo7twfj

