<script lang="ts">
  import Button from "./lib/ui/Button.svelte";
  import Modal from "./lib/ui/Modal.svelte";
  import Panel from "./lib/ui/Panel.svelte";
  import LicenceText from "./LicenceText.svelte";

  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();
</script>

<!-- doc-1 §10.3's D2 and D5: the conditions the user carries come before the full text, since
     they are easy to miss in it, and the text itself decides. -->
<Modal heading="ライセンス" {onclose}>
  <div class="sections">
    <Panel heading="利用者が守る条件">
      {#snippet children()}
        <div class="terms">
          <p>
            翻訳に使うモデル PLaMo 3 Translate は、Preferred Networks の PLaMo Community License で提供されています。このうち、使う人自身が守る主な条件は次のとおりです。正確な内容は下の全文によります。
          </p>
          <ul>
            <li>
              業務で使うとき (組織の中での利用や、製品・サービスの開発を含む) は、使う人自身が PFN の登録ページで事前に登録します。使う人かその関係会社の直近の事業年度の収入か売上が 10 億円を超えるときは、登録では足りず、別に商用のライセンスが要ります (第 3 条 (3))。私的な利用と学術の利用は、これに当たりません (第 1 条 (10))。
            </li>
            <li>
              訳文を書籍・論文など独立した長文の公表物の全部か実質的な部分として公表するときは、奥付・謝辞・末尾などに PLaMo を翻訳に使ったことを書きます (第 5 条 (2))。
            </li>
            <li>
              訳文を他の人へ渡すときや公開するときは、その訳文で学習したモデルにこの契約が適用されることを、受け取る人が分かる形で示します (第 5 条 (4))。
            </li>
            <li>法律・医療・金融・人物評価など重要な事項の決定で、訳文を唯一の根拠にしません (第 7 条 (2))。</li>
          </ul>
        </div>
      {/snippet}
    </Panel>

    <Panel heading="PLaMo Community License">
      {#snippet children(headingId)}
        <LicenceText labelledby={headingId} />
      {/snippet}
    </Panel>
  </div>

  {#snippet actions()}
    <Button label="閉じる" onpress={onclose} />
  {/snippet}
</Modal>

<style lang="scss">
  .sections {
    display: grid;
    gap: var(--snz-space-md);
  }

  .terms {
    display: grid;
    gap: var(--snz-space-sm);
    padding: var(--snz-space-sm) var(--snz-space-md);

    p,
    ul {
      margin: 0;
    }

    ul {
      display: grid;
      gap: var(--snz-space-xs);
      padding-inline-start: var(--snz-space-lg);
    }
  }
</style>
