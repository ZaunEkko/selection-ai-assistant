import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { MarkdownPreview } from '../components/MarkdownPreview';

describe('MarkdownPreview image privacy', () => {
  it.each([
    'https://tracker.example/pixel.gif?uid=123',
    'http://127.0.0.1/private.png',
    '/relative.png',
    'file:///C:/secret.png',
    'blob:https://example.com/image-id',
    'data:image/svg+xml;base64,PHN2Zz4=',
  ])('blocks image URL without creating an img element: %s', (url) => {
    render(<MarkdownPreview markdown={`![测试图](${url})`} />);

    expect(screen.queryByRole('img', { name: '测试图' })).not.toBeInTheDocument();
    expect(screen.getByText('远程图片已阻止：测试图')).toBeInTheDocument();
  });

  it('renders allowlisted base64 raster images', () => {
    const url = 'data:image/png;base64,aGVsbG8=';

    render(<MarkdownPreview markdown={`![内嵌图片](${url})`} />);

    expect(screen.getByRole('img', { name: '内嵌图片' })).toHaveAttribute('src', url);
    expect(screen.queryByText(/远程图片已阻止/)).not.toBeInTheDocument();
  });

  it('keeps normal HTTPS links user-triggered and isolated from the opener', () => {
    render(<MarkdownPreview markdown="[查看文档](https://example.com/docs)" />);

    expect(screen.getByRole('link', { name: '查看文档' })).toHaveAttribute('href', 'https://example.com/docs');
    expect(screen.getByRole('link', { name: '查看文档' })).toHaveAttribute('target', '_blank');
    expect(screen.getByRole('link', { name: '查看文档' })).toHaveAttribute('rel', 'noopener noreferrer');
  });
});
