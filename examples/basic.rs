extern crate shamir;

use shamir::SecretData;

fn main() {
    let secret_data = SecretData::with_secret(&"Hello World!"[..], 3);

    let share1 = secret_data.get_share(1);
    let share2 = secret_data.get_share(2);
    let share3 = secret_data.get_share(3);
    println!("Used keys:\n1:{:?}\n2:{:?}\n3:{:?}", share1, share2, share3);
    let recovered = SecretData::recover_secret(3, vec![share1, share2, share3]).unwrap();

    println!("Recovered {}", recovered);

}
