import { KeyringPair } from "@polkadot/keyring/types";
import { ISubmittableResult } from "@polkadot/types/types";
import { SubmittableExtrinsic } from "@polkadot/api/types";

export async function signSendAndWait(
    tx: SubmittableExtrinsic<"promise">,
    signer: KeyringPair
) {
    return new Promise<ISubmittableResult>((resolve, reject) =>
      tx.signAndSend(signer, (result) => {
        console.log(result)
        switch (true) {
          case result.isError:
            console.log("hi", { result })
            return reject(result.status);
          case result.isInBlock:
            return resolve(result);
          case result.isWarning:
            console.warn(result.toHuman(true));
        }
      })
    );
  }