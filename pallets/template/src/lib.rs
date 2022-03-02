#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use sp_std::vec::Vec;
	use frame_system::pallet_prelude::*;
	use frame_support::pallet_prelude::*;
	use frame_support::{
		sp_runtime::traits::{Hash, AccountIdConversion, SaturatedConversion},
		traits::{Currency, ExistenceRequirement},
	};

	#[cfg(feature = "std")]
	use frame_support::serde::{Deserialize, Serialize};

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;


	//Struct for Quiz
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Quiz<T:Config>{
		pub owner: AccountOf<T>,
		pub questions: Vec<Question>,
		pub rating: u8,
	}

	//Struct for Question
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Question{
		pub statement: Vec<u8>,
		pub option1: Vec<u8>,
		pub option2: Vec<u8>,
		pub option3: Vec<u8>,
		pub option4: Vec<u8>,
	}

	//Struct for Solution of a quiz --- a quiz is consist of 5 questions so the the solution will have 5 answers
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Solution{
		pub answer1: u8,
		pub answer2: u8,
		pub answer3: u8,
		pub answer4: u8,
		pub answer5: u8,
	}

	#[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

	#[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: Currency<Self::AccountId>;

		// the amount of tokens to deduct per wrong answer
		#[pallet::constant]
		type TokensPerQuestion: Get<u32>;
    }

	 // Errors.
	 #[pallet::error]
	 pub enum Error<T> {
		 /// If the option provided is invalid
		 InvalidOptionProvided,
		 /// Handles checking whether Quiz Exists
		 QuizDoesNotExist,
		 /// Handles if user rating not found (very rare)
		 UserRatingNotFound,
		 /// Handles if user rating is not enough for attempting the quiz
		 UserRatingTooLow,
		 /// Handles if the quiz owner tries to attempt the quiz
		 OwnerCannotAttemptQuiz,
		 /// Handles if the non quiz owner tries to change the quiz settings
		 NotTheQuizOwner,
		 /// If the quiz cannot be deleted
		 CannotDeleteQuiz,
		 /// If the player has not enough balance 
		 InsufficientBalance,
	 }
 
	 #[pallet::event]
	 #[pallet::generate_deposit(pub(super) fn deposit_event)]
	 pub enum Event<T: Config> {
		 /// A new quiz was created. \[QuizId, Account, Rating\]
		 QuizCreated(u64, T::AccountId, u8),
		 /// Score was generated after attempting the quiz. \[QuizID, AccountId, Score\]
		 QuizScore(u64, T::AccountId, u8),
		 /// Quiz was deleted in block number. \[BlockNumber\]
		 QuizDeleted(u64),
	 }
	 
	 #[pallet::storage]
	 #[pallet::getter(fn get_quiz)]
	 pub(super) type Quizzes<T:Config> = StorageMap<_, Twox64Concat, T::Hash, Quiz<T>>; // list of quizzes

	 #[pallet::storage]
	 #[pallet::getter(fn get_solution)]
	 pub(super) type Solutions<T:Config> = StorageMap<_, Twox64Concat, T::Hash, Solution>; // list of answers

	 #[pallet::storage]
	 #[pallet::getter(fn get_user_rating)]
	 pub(super) type UserRating<T:Config> = StorageMap<_, Twox64Concat, T::AccountId, u8, ValueQuery>;

	 #[pallet::storage]
	 #[pallet::getter(fn get_latest_quiz)]
	 pub(super) type QuizCnt<T:Config> = StorageValue<_, u64, ValueQuery>;

	 #[pallet::storage]
	 #[pallet::getter(fn get_quiz_to_delete)]
	 pub(super) type QuizToDelete<T:Config> = StorageMap<_, Twox64Concat, T::Hash, Vec<T::Hash>, ValueQuery>;

	 #[pallet::hooks]
	 impl<T:Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		 fn on_initialize(now: T::BlockNumber) -> Weight {
			 let total_weight : Weight = 10;
			 Self::check_and_delete_quiz(now);
			 total_weight
		 }
	 }

	 #[pallet::call]
    impl<T: Config> Pallet<T> {

		#[pallet::weight(100)]
		pub fn add_quiz(
			origin: OriginFor<T>,
			question1: Question,
			question2: Question,
			question3: Question,
			question4: Question,
			question5: Question,
			solution: Solution,
			rating: u8,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(solution.answer1 > 0 && solution.answer2 > 0 && solution.answer3 > 0 && solution.answer4 > 0, <Error<T>>::InvalidOptionProvided);
			ensure!(solution.answer1 <= 4 && solution.answer2 <= 4 && solution.answer3 <= 4 && solution.answer4 <= 4, <Error<T>>::InvalidOptionProvided);
			let mut _questions = Vec::new();
			_questions.push(question1);
			_questions.push(question2);
			_questions.push(question3);
			_questions.push(question4);
			_questions.push(question5);
			let quiz = Quiz::<T> {
				owner: sender.clone(),
				questions: _questions,
				rating: rating.clone(),
			};
			let quiz_count = Self::get_latest_quiz() + 1;
			let quiz_id = T::Hashing::hash_of(&quiz_count);
			<Quizzes<T>>::insert(quiz_id.clone(), quiz);
			<Solutions<T>>::insert(quiz_id, solution);
			<QuizCnt<T>>::put(quiz_count);

			let the_end_block_number = <frame_system::Pallet<T>>::block_number();
			Self::add_quiz_to_be_deleted(the_end_block_number, quiz_count)?;
			Self::deposit_event(Event::QuizCreated(quiz_count, sender, rating));
			Ok(())
		}

		#[pallet::weight(100)]
		pub fn attempt_quiz(
			origin: OriginFor<T>,
			quiz_count: u64,
			submission: Solution
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			
			let quiz_id = T::Hashing::hash_of(&quiz_count);
			let quiz = Self::get_quiz(&quiz_id).ok_or(<Error<T>>::QuizDoesNotExist)?;

			// ensuring the quiz attemptor is not the quiz creator
			ensure!(sender != quiz.owner,<Error<T>>::OwnerCannotAttemptQuiz);

			let user_rating = Self::get_user_rating(&sender);

			// ensure the user is qualified to attempt the quiz
			ensure!(user_rating >= quiz.rating - 1,<Error<T>>::UserRatingTooLow);

			let solution = Self::get_solution(&quiz_id).ok_or(<Error<T>>::QuizDoesNotExist)?;

			let score = Self::find_score(submission, solution);

			let user_rating = Self::get_user_rating(&sender);

			//the money feature 
			let token_to_pay = (5 - score as u32) * T::TokensPerQuestion::get();
			let token_to_pay : BalanceOf<T> = token_to_pay.into();
			Self::transfer_tokens_to_owner(&sender, &quiz.owner, token_to_pay)?;			
			Self::update_rating(sender.clone(), score.clone(), user_rating);

			Self::deposit_event(Event::QuizScore(quiz_count, sender, score));
			Ok(())
		}

		#[pallet::weight(100)]
		pub fn delete_quiz(
			origin: OriginFor<T>,
			quiz_count: u64
		) -> DispatchResult {
			// function body starts here
			let sender = ensure_signed(origin)?;
			let quiz_id = T::Hashing::hash_of(&quiz_count);
			let quiz = Self::get_quiz(&quiz_id).ok_or(<Error<T>>::QuizDoesNotExist)?;

			// ensuring that only the quiz owner can set the quiz for deletion
			ensure!(sender == quiz.owner, <Error<T>>::NotTheQuizOwner);
			<Quizzes<T>>::remove(quiz_id);
			Ok(())
			// function body ends here
		}
    }

	impl<T:Config> Pallet<T> {
		//Helper functions here

		pub fn find_score(
			submission: Solution,
			solution: Solution,
		) -> u8 {
			// function body starts here

			let mut score : u8 = 0;

			// checking for correct answers
			if submission.answer1 == solution.answer1 {
				score+=1;
			}
			if submission.answer2 == solution.answer2 {
				score+=1;
			}
			if submission.answer3 == solution.answer3 {
				score+=1;
			}
			if submission.answer4 == solution.answer4 {
				score+=1;
			}
			if submission.answer5 == solution.answer5 {
				score+=1;
			}
			score
			//function body ends here
		}

		// function to update the rating of the user
		pub fn update_rating(
			user: T::AccountId,
			current_score: u8,
			user_rating: u8,
		){
			// function body starts here
			let total : u8 = match user_rating {
				0 => 1,
				_ => 6
			};
			let user_rating = (user_rating * 5 + current_score)/total;
			<UserRating<T>>::insert(user, user_rating);
			// function body ends here
		}

		pub fn check_and_delete_quiz(
			block_number : T::BlockNumber
		){
			// function body starts here
			let block : u64 = block_number.saturated_into::<u64>();
			let block_hash = T::Hashing::hash_of(&block);
			let delete_vec = Self::get_quiz_to_delete(block_hash);
			for hash in delete_vec {
				<Quizzes<T>>::remove(hash);
				
				Self::deposit_event(Event::QuizDeleted(block.clone()));
			}
			//function body ends here
		}

		pub fn add_quiz_to_be_deleted(
			the_end_block_number : T::BlockNumber,
			quiz_number : u64,
		) -> Result<(), Error<T>> {
			// hook logic down 
			let mut  the_end_block_number = the_end_block_number.saturated_into::<u64>();
			// the_end_block_number = 14400 + the_end_block_number;  // this is for production
			the_end_block_number = 10 + the_end_block_number; // this is for the test
			let delete_id = T::Hashing::hash_of(&the_end_block_number);
			let quiz_id = T::Hashing::hash_of(&quiz_number);
			<QuizToDelete<T>>::try_mutate(&delete_id, |quiz_vec| { 
				quiz_vec.push(quiz_id);
				Ok(())
			})?;
			Ok(())
		}

		pub fn transfer_tokens_to_owner(
			sender: &T::AccountId,
			receiver: &T::AccountId,
			amount: BalanceOf<T>,
		) -> Result<(),Error<T>> {
			// ensure!(T::Currency::free_balance(&sender) >= amount, <Error<T>>::InsufficientBalance)?;
			let free_balance = T::Currency::free_balance(&sender);
			if free_balance < amount {
				Err(<Error<T>>::InsufficientBalance)
			}else{
				T::Currency::transfer(&sender, &receiver, amount, ExistenceRequirement::KeepAlive).map_err(|_|<Error<T>>::InsufficientBalance)?;
				Ok(())
			}
		}
	}
}