-- Your SQL goes here
CREATE TABLE Exercises (
  id SERIAL PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  equipment TEXT,
  primary_muscle TEXT,
  secondary_muscle TEXT,
  description TEXT
);

CREATE TABLE Tags (
  id SERIAL PRIMARY KEY,
  name TEXT UNIQUE NOT NULL
);

CREATE TABLE Workouts (
  id SERIAL PRIMARY KEY,
  name TEXT,
  performed_at TIMESTAMP DEFAULT NOW(),
  notes TEXT
);

CREATE TABLE Sets (
  id SERIAL PRIMARY KEY,
  exercise_id INT NOT NULL REFERENCES Exercises(id) ON DELETE CASCADE,
  workout_id INT NOT NULL REFERENCES Workouts(id) ON DELETE CASCADE,
  weight NUMERIC(6,2) NOT NULL,
  reps INT NOT NULL,
  rpe NUMERIC(1,2),
  set_number INT
);

CREATE TABLE SetTags (
  id SERIAL PRIMARY KEY,
  set_id INT NOT NULL REFERENCES Sets(id) ON DELETE CASCADE,
  tag_id INT NOT NULL REFERENCES Tags(id) ON DELETE CASCADE
);

