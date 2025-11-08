-- Your SQL goes here
CREATE TABLE ExerciseTags (
  id SERIAL PRIMARY KEY,
  exercise_id INT NOT NULL REFERENCES Exercises(id) ON DELETE CASCADE,
  tag_id INT NOT NULL REFERENCES Tags(id) ON DELETE CASCADE,
  UNIQUE(exercise_id, tag_id)
);
