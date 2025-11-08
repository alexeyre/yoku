-- Your SQL goes here
-- Insert common exercises
INSERT INTO Exercises (name, equipment, primary_muscle, secondary_muscle, description) VALUES
('Push-up', NULL, 'Chest', 'Triceps', 'A bodyweight pressing exercise.'),
('Pull-up', 'Pull-up bar', 'Back', 'Biceps', 'A vertical pulling exercise.'),
('Squat', 'Barbell', 'Quadriceps', 'Glutes', 'A lower body compound lift.'),
('Deadlift', 'Barbell', 'Hamstrings', 'Back', 'A posterior chain compound lift.'),
('Bench Press', 'Barbell', 'Chest', 'Triceps', 'A pressing exercise on a bench.'),
('Overhead Press', 'Barbell', 'Shoulders', 'Triceps', 'A vertical pressing exercise.'),
('Bicep Curl', 'Dumbbell', 'Biceps', NULL, 'Isolation exercise for biceps.'),
('Tricep Extension', 'Dumbbell', 'Triceps', NULL, 'Isolation exercise for triceps.'),
('Plank', NULL, 'Core', NULL, 'Static core hold exercise.'),
('Lunge', NULL, 'Quadriceps', 'Glutes', 'Unilateral lower body exercise.');

-- Insert common tags
INSERT INTO Tags (name) VALUES
('left-side'),
('right-side'),
('both-sides'),
('slow and controlled'),
('to failure'),
('sick'),
('warm-up'),
('cool-down'),
('explosive'),
('assistance');

